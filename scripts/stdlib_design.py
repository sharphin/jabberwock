#!/usr/bin/env python3
"""Validate/generate the stdlib v1 design artifacts; this implements no runtime APIs."""
import argparse
import copy
import hashlib
import json
import math
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'docs' / 'stdlib-v1'
PRIMITIVES = {'unit': (1, 1, 'uint8_t'), 'bool': (4, 4, 'uint32_t'),
              'u8': (1, 1, 'uint8_t'), 'u16': (2, 2, 'uint16_t'),
              'u32': (4, 4, 'uint32_t'), 'u64': (8, 8, 'uint64_t'),
              'i64': (8, 8, 'int64_t'), 'f64': (8, 8, 'double')}
CAPS = {'fs', 'net', 'tls_client', 'postgres', 'window', 'draw2d', 'image', 'gui'}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pairs(items):
    out = {}
    for key, value in items:
        require(key not in out, 'duplicate JSON key: ' + key)
        out[key] = value
    return out


def read_json(path):
    return json.loads(path.read_text(encoding='utf-8'), object_pairs_hook=pairs,
                      parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))


def encoded(obj):
    return (json.dumps(obj, ensure_ascii=False, sort_keys=True, separators=(',', ':'))).encode('utf-8')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def keys(obj, required, optional=()):
    require(isinstance(obj, dict), 'expected object')
    require(set(required) <= set(obj) <= set(required) | set(optional), 'unknown/missing keys: ' + str(set(obj) ^ set(required)))


def split_type(text):
    require(isinstance(text, str), 'type must be string')
    if '<' not in text:
        return text, []
    require(text.endswith('>'), 'bad generic type: ' + text)
    kind, inner = text.split('<', 1)
    inner = inner[:-1]
    parts, start, depth = [], 0, 0
    for i, c in enumerate(inner):
        depth += (c == '<') - (c == '>')
        require(depth >= 0, 'unbalanced type')
        if c == ',' and depth == 0:
            parts.append(inner[start:i]); start = i + 1
    parts.append(inner[start:])
    require(depth == 0, 'unbalanced type')
    return kind, parts


def children(text, types):
    kind, args = split_type(text)
    if args:
        require(kind in {'option', 'list', 'result', 'array'}, 'unknown generic ' + kind)
        require(len(args) == (2 if kind in {'result', 'array'} else 1), 'generic arity')
        if kind == 'array':
            require(args[1].isdigit() and 0 < int(args[1]) <= 65536, 'array size')
            return [args[0]]
        return args
    require(kind in PRIMITIVES or kind == 'str' or kind in types, 'unknown type ' + kind)
    if kind not in types:
        return []
    desc = types[kind]
    if desc['kind'] in {'record', 'opaque'}:
        return [f['type'] for f in desc['fields']]
    if desc['kind'] == 'enum':
        return [f['type'] for v in desc['variants'] for f in v['fields']]
    return []


def slots(text, types):
    kind, args = split_type(text)
    if kind in {'list', 'array'}:
        require(slots(args[0], types) == 0, 'resource-bearing variable array at C boundary: ' + text)
        return 0
    if kind in {'option', 'result'}:
        return max(slots(x, types) for x in args)
    if kind not in types:
        return 0
    desc = types[kind]
    if desc['kind'] == 'resource':
        return 1
    if desc['kind'] == 'enum':
        return max((sum(slots(f['type'], types) for f in v['fields']) for v in desc['variants']), default=0)
    return sum(slots(f['type'], types) for f in desc['fields'])


def fields(items):
    names = []
    for item in items:
        keys(item, ['name', 'type'])
        require(re.fullmatch(r'[a-z][a-z0-9_]*', item['name']), 'invalid field name')
        names.append(item['name'])
    require(len(names) == len(set(names)), 'duplicate field')


def value_ok(value, text, types, params=None):
    if isinstance(value, dict) and set(value) == {'$param'}:
        require(params and params.get(value['$param']) == text, 'invalid default parameter')
        return
    kind, args = split_type(text)
    if kind == 'option':
        if value is not None: value_ok(value, args[0], types, params)
    elif kind in {'list', 'array'}:
        require(isinstance(value, list), 'default list')
        if kind == 'array': require(len(value) == int(args[1]), 'default array size')
        for x in value: value_ok(x, args[0], types, params)
    elif kind in {'u8', 'u16', 'u32', 'u64', 'i64'}:
        bits = int(kind[1:]); lo = -(1 << (bits-1)) if kind[0] == 'i' else 0
        hi = (1 << (bits-1)) - 1 if kind[0] == 'i' else (1 << bits) - 1
        require(type(value) is int and lo <= value <= hi, 'integer default range')
    elif kind == 'bool': require(type(value) is bool, 'bool default')
    elif kind == 'f64': require(type(value) in {int, float} and math.isfinite(value), 'finite default')
    elif kind == 'str': require(isinstance(value, str), 'string default')
    elif kind == 'unit': require(value is None, 'unit default')
    elif kind in types:
        desc = types[kind]
        if desc['kind'] == 'record':
            keys(value, [f['name'] for f in desc['fields']])
            for f in desc['fields']: value_ok(value[f['name']], f['type'], types, params)
        elif desc['kind'] == 'enum':
            keys(value, ['variant', 'fields'])
            found = [v for v in desc['variants'] if v['name'] == value['variant']]
            require(len(found) == 1, 'unknown default variant')
            keys(value['fields'], [f['name'] for f in found[0]['fields']])
            for f in found[0]['fields']: value_ok(value['fields'][f['name']], f['type'], types, params)
        else: raise ValueError('opaque/resource literal default')
    else: raise ValueError('unsupported default type ' + text)


def validate(spec):
    keys(spec, ['schema_version', 'api_version', 'resource_abi', 'codec_version', 'state', 'modules', 'types', 'operations', 'reserved_operation_ids'])
    require(all(type(spec[k]) is int and spec[k] == 1 for k in ['schema_version', 'api_version', 'resource_abi', 'codec_version']), 'unsupported version')
    require(spec['state'] == 'design-only', 'this is a design registry')
    types = spec['types']; modules = {}; ids = set(); symbols = set(); public = set()
    banks={'std.fs':1,'std.time':2,'std.net':3,'std.tls':4,'postgres':5,'window':6,'draw2d':7,'image':8,'gui':9}
    for module in spec['modules']:
        keys(module, ['name', 'source', 'capabilities', 'dependencies'])
        require(module['name'] not in modules, 'duplicate module')
        require(module['source'] in {'standard', 'bundled'}, 'module source')
        require(set(module['capabilities']) <= CAPS, 'unknown capability')
        require(len(set(module['capabilities']))==len(module['capabilities']) and len(set(module['dependencies']))==len(module['dependencies']),'duplicate capability/dependency')
        modules[module['name']] = module
    visiting, done = set(), set()
    def visit(name):
        require(name not in visiting, 'module/type cycle: ' + name)
        if name in done: return
        require(name in modules, 'unknown module dependency ' + name)
        visiting.add(name)
        for dep in modules[name]['dependencies']: visit(dep)
        visiting.remove(name); done.add(name)
    for name in modules: visit(name)
    owner = lambda name: max((m for m in modules if name.startswith(m+'.')), key=len, default=None)
    kinds = []; value_ids = set()
    for name, desc in types.items():
        require(owner(name), 'type has no module ' + name)
        kind = desc.get('kind')
        if kind == 'resource':
            keys(desc, ['kind', 'resource_kind', 'copy', 'send'])
            require(desc['copy'] is False and desc['send'] is False, 'resource cannot Copy/Send')
            require(type(desc['resource_kind']) is int and 1 <= desc['resource_kind'] <= 12,'resource kind range')
            kinds.append(desc['resource_kind'])
        elif kind == 'record': keys(desc, ['kind', 'fields'], ['default']); fields(desc['fields'])
        elif kind == 'opaque':
            keys(desc, ['kind', 'copy', 'send', 'value_id', 'fields']); fields(desc['fields'])
            require(type(desc['copy']) is bool and type(desc['send']) is bool, 'opaque traits')
            require(type(desc['value_id']) is int and 0 < desc['value_id'] < 2**32 and desc['value_id'] not in value_ids, 'opaque type ID')
            value_ids.add(desc['value_id'])
        elif kind == 'enum':
            keys(desc, ['kind', 'variants']); names = set(); tags = set()
            for v in desc['variants']:
                keys(v, ['name', 'tag', 'fields']); fields(v['fields'])
                require(v['name'] not in names and v['tag'] not in tags and type(v['tag']) is int and 0 <= v['tag'] < 2**32, 'enum tag/name')
                names.add(v['name']); tags.add(v['tag'])
        else: raise ValueError('unknown type class')
    require(sorted(kinds) == list(range(1, 13)), 'resource kind assignment')
    active, seen = set(), set()
    def visit_type(text):
        require(text not in active, 'recursive value type ' + text)
        if text in seen: return
        active.add(text)
        for t in children(text, types): visit_type(t)
        active.remove(text); seen.add(text)
    def visibility(text, module):
        kind, args = split_type(text)
        for arg in (args[:1] if kind == 'array' else args): visibility(arg, module)
        if kind in types:
            require(owner(kind) in [module] + modules[module]['dependencies'], 'undeclared API type dependency: ' + kind + ' in ' + module)
    for name, desc in types.items():
        visit_type(name)
        for child in children(name, types): visibility(child, owner(name))
        if desc['kind'] == 'opaque': require(slots(name, types) == 0, 'resource hidden in opaque data')
        if 'default' in desc: value_ok(desc['default'], name, types)
    for operation in spec['operations']:
        keys(operation, ['id', 'module', 'name', 'symbol', 'params', 'returns', 'effects', 'barrier', 'capabilities'], ['default_result'])
        ident = operation['id']; module = operation['module']; name = module + '.' + operation['name']
        require(type(ident) is int and 0 < ident < 2**32 and ident not in ids, 'operation ID collision/range')
        require(ident not in spec['reserved_operation_ids'], 'retired operation ID reused')
        require(module in modules and name not in public, 'public symbol collision')
        require(ident >> 16 == banks[module], 'wrong operation bank')
        require(re.fullmatch(r'jw_[a-z0-9_]+_v1', operation['symbol']) and operation['symbol'] not in symbols, 'C symbol collision')
        require(set(operation['effects']) <= {'read-os', 'write-os', 'clock', 'gui'} and operation['barrier'] is True, 'effect/barrier')
        require(operation['capabilities'] == modules[module]['capabilities'], 'capability disagreement')
        param_names = set()
        for param in operation['params']:
            keys(param, ['name', 'type', 'mode'])
            require(re.fullmatch(r'[a-z][a-z0-9_]*', param['name']) and param['name'] not in param_names, 'argument name')
            require(param['mode'] in {'value', 'ref', 'mut', 'move'}, 'argument access')
            visit_type(param['type']); visibility(param['type'], module)
            if param['type'] in types and types[param['type']]['kind'] == 'resource': require(param['mode'] != 'value', 'resource copied')
            param_names.add(param['name'])
        visit_type(operation['returns']); visibility(operation['returns'], module)
        require(slots(operation['returns'], types) <= 4, 'too many output owner slots')
        if 'default_result' in operation:
            require(operation['effects'] == [], 'default factory has external effects')
            value_ok(operation['default_result'], operation['returns'], types, {p['name']:p['type'] for p in operation['params']})
        ids.add(ident); symbols.add(operation['symbol']); public.add(name)
    return modules


def cfield(text, types):
    if text in PRIMITIVES: return PRIMITIVES[text]
    if text in types and types[text]['kind'] == 'resource': return 24, 8, 'JwResTokenV1'
    return 16, 8, 'JwSliceV1'


def header(spec):
    lines = ['/* Generated design declarations only; no runtime implementation. */', '#ifndef JW_STDLIB_ABI_V1_H', '#define JW_STDLIB_ABI_V1_H', '#include <stdint.h>', '#include <stddef.h>', '#ifdef __cplusplus', '#define JW_SA static_assert', '#define JW_ALIGN alignof', 'extern "C" {', '#else', '#define JW_SA _Static_assert', '#define JW_ALIGN _Alignof', '#endif',
             'typedef struct { uint64_t slot, generation; uint32_t kind, access; } JwResTokenV1;',
             'typedef struct { const uint8_t *data; uint64_t length; } JwSliceV1;',
             'typedef struct { uint8_t *data; uint64_t length, capacity; } JwBufferV1;',
             'typedef struct { uint32_t valid, effect; uint64_t progress; uint32_t outcome, reserved; JwBufferV1 value; } JwOutcomeV1;',
             'JW_SA(sizeof(void *) == 8, "64-bit ABI required");',
             'JW_SA(sizeof(JwResTokenV1) == 24 && JW_ALIGN(JwResTokenV1) == 8, "token layout");',
             'JW_SA(offsetof(JwResTokenV1, generation) == 8 && offsetof(JwResTokenV1, kind) == 16 && offsetof(JwResTokenV1, access) == 20, "token offsets");',
             'JW_SA(sizeof(JwSliceV1) == 16 && sizeof(JwBufferV1) == 24, "buffer layout");',
             'JW_SA(sizeof(JwOutcomeV1) == 48 && offsetof(JwOutcomeV1, value) == 24 && offsetof(JwOutcomeV1, outcome) == 16, "outcome layout");',
             'int32_t jw_res_require_abi_v1(uint32_t abi, const uint8_t *registry_sha256_32);',
             'int32_t jw_res_context_new_v1(uint32_t abi, void **out_context);',
             'int32_t jw_res_context_finish_v1(void *context);',
             'int32_t jw_res_close_v1(void *context, JwResTokenV1 *inout_owner);',
             'int32_t jw_res_drop_v1(void *context, JwResTokenV1 *inout_owner);',
             'int32_t jw_buffer_release_v1(void *context, JwBufferV1 *inout_buffer);']
    layouts = []
    for operation in spec['operations']:
        ident = f'{operation["id"]:08x}'; a = 'JwArgs_' + ident + 'V1'; r = 'JwOut_' + ident + 'V1'
        lines.append('/* ' + operation['module']+'.'+operation['name'] + ' */')
        members, offset, align, checks = [], 0, 1, []
        for param in operation['params']:
            size, al, ct = cfield(param['type'], spec['types']); offset = (offset+al-1)//al*al
            members.append(ct+' '+param['name']+';'); checks.append((param['name'], offset)); offset += size; align=max(align,al)
        if not members: members=['uint64_t reserved;']; offset=8; align=8
        size = (offset+align-1)//align*align
        lines.append('typedef struct { '+' '.join(members)+' } '+a+';')
        owners = slots(operation['returns'], spec['types'])
        lines.append('typedef struct { JwOutcomeV1 result; '+' '.join('JwResTokenV1 resource_'+str(i)+';' for i in range(owners))+' } '+r+';')
        lines.append('int32_t '+operation['symbol']+'(void *context, '+a+' *inout_args, '+r+' *out);')
        lines.append(f'JW_SA(sizeof({a}) == {size} && JW_ALIGN({a}) == {align}, "args {ident}");')
        for field, off in checks: lines.append(f'JW_SA(offsetof({a}, {field}) == {off}, "field {ident}.{field}");')
        lines.append(f'JW_SA(sizeof({r}) == {48+24*owners}, "out {ident}");')
        layouts.append((operation, size, align, owners))
    lines += ['#ifdef __cplusplus', '}', '#endif', '#undef JW_SA', '#undef JW_ALIGN', '#endif', '']
    return '\n'.join(lines), layouts


def schema_object(properties, required=None):
    return {'type':'object','properties':properties,'required':list(properties) if required is None else required,'additionalProperties':False}


def catalog_artifacts(spec):
    string = {'type':'string','minLength':1,'maxLength':16384}; digest={'type':'string','pattern':'^[0-9a-f]{64}$'}
    path={'type':'string','pattern':r'^(?!/)(?!.*(?:^|/)\.{1,2}(?:/|$))(?!.*//)[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*$','maxLength':1024}
    source=schema_object({'path':path,'bytes':{'type':'integer','minimum':0,'maximum':16777216},'sha256':digest})
    names={'type':'array','items':string,'uniqueItems':True,'maxItems':16384}
    capability={'type':'array','items':{'enum':sorted(CAPS)},'uniqueItems':True,'maxItems':8}
    mod=schema_object({'id':string,'visibility':{'enum':['public','private']},'source':source,'imports':names,'allowed_importers':names,'capabilities':capability,'resource_abi':{'const':1}})
    pkg=schema_object({'name':string,'root':path,'manifest':source,'entry':source,'dependencies':names,'standard_modules':names,'capabilities':capability,'resource_abi':{'const':1}})
    schema=schema_object({'catalog_version':{'const':1},'catalog_kind':{'enum':['design-example','installed']},'distribution_id':string,'stdlib_build_id':digest,'registry_sha256':digest,'runtime_sha256':digest,'runtime_manifest':source,'resource_abi':{'const':1},'targets':{'type':'array','items':{'enum':['x86_64-unknown-linux-gnu','x86_64-pc-windows-msvc']},'uniqueItems':True,'minItems':1},'modules':{'type':'array','items':mod,'maxItems':16384},'packages':{'type':'array','items':pkg,'maxItems':4096}})
    schema={'$schema':'https://json-schema.org/draft/2020-12/schema',**schema}
    def src(path):
        content=('// design example only: '+path+'\n').encode()
        return {'path':path,'bytes':len(content),'sha256':sha(content)}
    runtime=sorted([src('runtime/'+n) for n in ['stdlib-abi-v1.h','jw_base.c','jw_fs.c','jw_net.c','jw_tls.c','jw_pg.c','jw_window.c','jw_draw2d.c','jw_image.c','jw_gui.c']],key=lambda s:s['path'])
    runtime_text=json.dumps(runtime,ensure_ascii=False,indent=2)+'\n'
    example={'catalog_version':1,'catalog_kind':'design-example','distribution_id':'jabberwock-stdlib-v1-design','registry_sha256':sha((BASE/'registry.json').read_bytes()),'runtime_sha256':sha(encoded(runtime)),'runtime_manifest':{'path':'runtime/manifest.json','bytes':len(runtime_text.encode()),'sha256':sha(runtime_text.encode())},'resource_abi':1,'targets':['x86_64-unknown-linux-gnu','x86_64-pc-windows-msvc'],'modules':[],'packages':[]}
    for module in spec['modules']:
        name=module['name']
        if module['source']=='standard':
            example['modules'].append({'id':name,'visibility':'public','source':src('std/'+name[4:]+'.jw'),'imports':module['dependencies']+['std._value'],'allowed_importers':[],'capabilities':module['capabilities'],'resource_abi':1})
        else:
            root='packages/'+name
            example['packages'].append({'name':name,'root':root,'manifest':src(root+'/alice.toml'),'entry':src(root+'/src/lib.jw'),'dependencies':[d for d in module['dependencies'] if not d.startswith('std.')],'standard_modules':[d for d in module['dependencies'] if d.startswith('std.')],'capabilities':module['capabilities'],'resource_abi':1})
    example['modules'].append({'id':'std._value','visibility':'private','source':src('std/_value.jw'),'imports':[],'allowed_importers':[m['name'] for m in spec['modules'] if m['source']=='standard'],'capabilities':[],'resource_abi':1})
    example['stdlib_build_id']=sha(b'JW-STDLIB-CATALOG-V1\0'+encoded(example))
    return schema,example,runtime_text


def check_catalog(value, schema, registry, installed=False):
    def check(v,s):
        if 'const' in s: require(type(v) is type(s['const']) and v==s['const'], 'catalog constant')
        if 'enum' in s: require(v in s['enum'], 'catalog enum')
        typ=s.get('type')
        if typ=='object':
            keys(v,s['required'])
            for k,x in v.items():check(x,s['properties'][k])
        elif typ=='array':
            require(isinstance(v,list) and s.get('minItems',0)<=len(v)<=s.get('maxItems',65536),'catalog array')
            if s.get('uniqueItems'):require(len({encoded(x) for x in v})==len(v),'catalog duplicate array')
            for x in v:check(x,s['items'])
        elif typ=='string':
            require(isinstance(v,str) and s.get('minLength',0)<=len(v)<=s.get('maxLength',16384),'catalog string')
            if 'pattern' in s:require(re.search(s['pattern'],v),'catalog pattern')
        elif typ=='integer':require(type(v) is int and s.get('minimum',0)<=v<=s.get('maximum',2**64-1),'catalog integer')
    check(value,schema)
    if installed: require(value['catalog_kind']=='installed','design fixture is not installable')
    body=copy.deepcopy(value); declared=body.pop('stdlib_build_id')
    require(declared==sha(b'JW-STDLIB-CATALOG-V1\0'+encoded(body)),'catalog build ID mismatch')
    require(value['registry_sha256']==sha((BASE/'registry.json').read_bytes()),'registry digest mismatch')
    mods={m['id']:m for m in value['modules']}; pkgs={p['name']:p for p in value['packages']}
    require(len(mods)==len(value['modules']) and len(pkgs)==len(value['packages']),'catalog duplicate ID')
    for module in value['modules']:
        require(module['id'].startswith('std.'),'invalid standard namespace')
        for name in module['imports']+module['allowed_importers']:require(name in mods,'dangling catalog module')
        if module['visibility']=='public':require(not module['allowed_importers'],'public whitelist')
        for dep in module['imports']:
            require(mods[dep]['visibility']=='public' or module['id'] in mods[dep]['allowed_importers'],'private catalog import denied')
    paths=[]
    for source in [value['runtime_manifest']]+[m['source'] for m in mods.values()]+[s for p in pkgs.values() for s in [p['manifest'],p['entry']]]:
        require(all(part not in {'','.','..'} for part in source['path'].split('/')),'unsafe catalog relative path')
        paths.append(source['path'])
    require(len(paths)==len(set(paths)),'duplicate source path')
    for pkg in pkgs.values():
        for dep in pkg['dependencies']:require(dep in pkgs,'unknown package dependency')
        for dep in pkg['standard_modules']:require(dep in mods and mods[dep]['visibility']=='public','private package import')
        require(pkg['manifest']['path'].startswith(pkg['root']+'/') and pkg['entry']['path'].startswith(pkg['root']+'/'),'package root escape')
    for module in registry['modules']:
        if module['source']=='standard':
            require(module['name'] in mods,'missing public module')
            actual=mods[module['name']]
            require(actual['visibility']=='public' and actual['capabilities']==module['capabilities'],'public module capability mismatch')
            require({d for d in actual['imports'] if mods[d]['visibility']=='public'}==set(module['dependencies']),'public module dependency mismatch')
        else:
            require(module['name'] in pkgs,'missing package')
            actual=pkgs[module['name']]
            require(actual['capabilities']==module['capabilities'],'package capability mismatch')
            require(set(actual['dependencies']+actual['standard_modules'])==set(module['dependencies']),'package dependency mismatch')
    graph={m['id']:m['imports'] for m in mods.values()}
    graph.update({p['name']:p['dependencies']+p['standard_modules'] for p in pkgs.values()})
    active,done=set(),set()
    def visit(name):
        require(name not in active,'catalog cycle')
        if name in done:return
        active.add(name)
        for dep in graph[name]:visit(dep)
        active.remove(name);done.add(name)
    for name in graph:visit(name)


def artifacts(spec):
    head,layouts=header(spec)
    schema,example,runtime_text=catalog_artifacts(spec)
    api=['# 標準ライブラリ公開API v1','', '状態: 実装前の契約。[registry.json](stdlib-v1/registry.json) から生成する。機能の提供・試験成功を意味しない。', '',
         '型式のlist<T>/array<T,N>/option<T>/result<T,E>はそれぞれT[]/T[N]/Option<T>/Result<T,E>に対応する契約表記。',
         'valueは複製可能な値、moveは値渡し（Copy型なら既存規則どおりコピー）、ref/mutは読取/可変借用。資源は移すか借り、複製しない。',
         'default factoryは明記した値だけを生成し、未指定の接続情報や秘密を補完しない。型名・field名・引数順・返却型の正本はJSON。', '',
         '[意味・既定値・上限](stdlib-v1-contracts.md)、[C境界](native-operations.md)、[登録と配布](catalog-registry.md) を併せて適用する。', '']
    for module in spec['modules']:
        m=module['name'];api+=['## '+m,'','| 操作 | 引数（順序・権限・型） | 戻り値 |','|---|---|---|']
        for op in spec['operations']:
            if op['module']==m:api.append('| `'+op['name']+'` | `'+(', '.join(p['mode']+' '+p['name']+': '+p['type'] for p in op['params']) or 'なし')+'` | `'+op['returns']+'` |')
        api+=['','| 型 | 定義 |','|---|---|']
        for name,d in spec['types'].items():
            if name.startswith(m+'.'):
                if d['kind']=='resource':definition='resource kind '+str(d['resource_kind'])+'; !Copy/!Send'
                elif d['kind']=='enum':definition='; '.join(v['name']+'('+', '.join(f['name']+': '+f['type'] for f in v['fields'])+') [tag '+str(v['tag'])+']' for v in d['variants'])
                else:definition=d['kind']+' { '+', '.join(f['name']+': '+f['type'] for f in d['fields'])+' }'+(' Copy='+str(d['copy'])+', Send='+str(d['send']) if d['kind']=='opaque' else '')
                if 'default' in d: definition += '; defaults='+json.dumps(d['default'],ensure_ascii=False,separators=(',',':'))
                api.append('| `'+name+'` | `'+definition+'` |')
        api.append('')
    ops=['# 個別操作のC ABI v1','', '状態: 実装前の設計。宣言と配置の正本は [生成ヘッダー](stdlib-v1/stdlib-abi-v1.h)。実装したC関数の一覧ではない。', '',
         'すべての公開操作は固定IDと型付きの専用Args/Outを持つ。呼出し規則とcodec、所有、失敗は [C境界の契約](stdlib-v1-contracts.md) に従う。', '',
         '| ID | 公開操作 | C symbol | Argsのbyte/整列 | Outのbyte | 所有slot数 |', '|---|---|---|---:|---:|---:|']
    for op,size,align,owners in layouts:ops.append(f'| 0x{op["id"]:08x} | {op["module"]}.{op["name"]} | {op["symbol"]} | {size}/{align} | {48+24*owners} | {owners} |')
    ops += ['', '## 不透明な値のcodec ID', '', '| ID | 型 | Copy/Send |', '|---:|---|---|']
    for name,d in spec['types'].items():
        if d['kind']=='opaque':ops.append(f'| {d["value_id"]} | {name} | {d["copy"]}/{d["send"]} |')
    return {'docs/public-apis.md':'\n'.join(api).rstrip()+'\n','docs/native-operations.md':'\n'.join(ops)+'\n','docs/stdlib-v1/stdlib-abi-v1.h':head,
            'docs/stdlib-v1/catalog.schema.json':json.dumps(schema,ensure_ascii=False,indent=2)+'\n',
            'docs/stdlib-v1/catalog.example.json':json.dumps(example,ensure_ascii=False,indent=2)+'\n',
            'docs/stdlib-v1/runtime.manifest.example.json':runtime_text}


def main():
    ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');args=ap.parse_args()
    spec=read_json(BASE/'registry.json');validate(spec)
    digest=sha(encoded(spec));lock=BASE/'registry.lock.json'
    if lock.exists():require(read_json(lock)=={'api_version':1,'contract_sha256':digest},'v1 contract changed: create a versioned successor, do not silently rewrite the lock')
    else:require(args.write,'missing contract lock')
    generated=artifacts(spec)
    if args.write:
        for name,content in generated.items(): (ROOT/name).write_bytes(content.encode('utf-8'))
        if not lock.exists():lock.write_bytes((json.dumps({'api_version':1,'contract_sha256':digest},indent=2)+'\n').encode('utf-8'))
    else:
        for name,content in generated.items():require((ROOT/name).read_text(encoding='utf-8')==content,'outdated generated file '+name)
    schema=read_json(BASE/'catalog.schema.json');example=read_json(BASE/'catalog.example.json');check_catalog(example,schema,spec)
    raw=(BASE/'runtime.manifest.example.json').read_bytes();manifest=read_json(BASE/'runtime.manifest.example.json')
    require(len(raw)==example['runtime_manifest']['bytes'] and sha(raw)==example['runtime_manifest']['sha256'],'runtime manifest raw content mismatch')
    require(sha(encoded(manifest))==example['runtime_sha256'],'runtime manifest content ID mismatch')
    paths=[s['path'] for s in manifest]
    require(paths==sorted(set(paths)) and example['runtime_manifest']['path'] not in paths,'runtime manifest order/recursion')
    rejected=0
    for mutate in [lambda s:s['operations'].append(copy.deepcopy(s['operations'][0])),
                   lambda s:s['types']['std.fs.File'].update(send=True),
                   lambda s:s['operations'][0].update(returns='missing.Type'),
                   lambda s:s['operations'][0].update(extra='ignored'),
                   lambda s:s['modules'][0]['dependencies'].append('std.fs')]:
        bad=copy.deepcopy(spec);mutate(bad)
        try:validate(bad)
        except ValueError:rejected+=1
        else:raise ValueError('invalid registry accepted')
    for mutate in [lambda s:s.update(resource_abi=2),lambda s:s['modules'][0]['source'].update(path='../escape.jw'),lambda s:s['packages'][0]['dependencies'].append('missing'),lambda s:s.update(extra='ignored')]:
        bad=copy.deepcopy(example);mutate(bad)
        body=copy.deepcopy(bad);body.pop('stdlib_build_id',None)
        bad['stdlib_build_id']=sha(b'JW-STDLIB-CATALOG-V1\0'+encoded(body))
        try:check_catalog(bad,schema,spec)
        except ValueError:rejected+=1
        else:raise ValueError('invalid catalog accepted')
    try:check_catalog(example,schema,spec,installed=True)
    except ValueError:rejected+=1
    else:raise ValueError('design example accepted for installation')
    print(f'{len(spec["types"])} types, {len(spec["operations"])} operations; generated artifacts match; {rejected} invalid inputs rejected')


if __name__=='__main__':main()
