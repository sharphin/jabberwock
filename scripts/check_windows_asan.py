#!/usr/bin/env python3
"""Verify Windows ASan can both start and detect a deliberate heap use after free."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

def main():
    selected = shutil.which(os.environ.get("JW_CLANG", "clang"))
    print("Selected clang: " + str(selected), flush=True)
    # Record likely DLL candidates without copying DLLs or changing machine settings.
    directories = ([str(Path(selected).parent)] if selected else []) + os.environ.get("PATH", "").split(os.pathsep)
    seen = set()
    for directory in directories:
        if directory and directory not in seen:
            seen.add(directory)
            for dll in Path(directory).glob("*asan*.dll"):
                print("ASan DLL candidate: " + str(dll), flush=True)
    with tempfile.TemporaryDirectory(prefix="jw-asan-preflight-") as d:
        folder = Path(d)
        for name, source in (("startup", "int main(void) { return 0; }"),
                             ("negative", "#include <stdlib.h>\nint main(void) { volatile int *p = (int*)malloc(sizeof(int)); *p = 7; free((void*)p); return *p; }")):
            c = folder / (name + ".c")
            exe = folder / (name + ".exe")
            c.write_text(source, encoding="utf-8")
            command = [os.environ.get("JW_CLANG", "clang"), "-O1", "-fsanitize=address", str(c), "-o", str(exe)]
            built = subprocess.run(command, capture_output=True, text=True, errors="replace", timeout=120)
            if built.returncode:
                print(built.stderr, file=sys.stderr)
                return 2
            result = subprocess.run([str(exe)], capture_output=True, text=True, errors="replace", timeout=30)
            code = result.returncode & 0xffffffff
            if code in (0xc0000135, 0xc0000139, 0xc000007b):
                print(f"ASan loader failure 0x{code:08X}: check matching LLVM ASan DLLs and PATH; safety code was not executed", file=sys.stderr)
                return 2
            if name == "startup" and result.returncode != 0:
                print(f"ASan startup failed: {result.returncode}: {result.stderr}", file=sys.stderr)
                return 2
            if name == "negative" and not (result.returncode != 0 and "AddressSanitizer: heap-use-after-free" in result.stderr):
                print(f"ASan did not detect the required defect: {result.returncode}: {result.stderr}", file=sys.stderr)
                return 2
    print("ASan startup and negative control passed")
    return 0

if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, subprocess.TimeoutExpired) as error:
        print(str(error), file=sys.stderr)
        sys.exit(2)
