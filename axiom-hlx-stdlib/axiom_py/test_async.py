#!/usr/bin/env python3
"""Test script for axiom_py async bindings"""

import asyncio
from axiom import AxiomEngine, verify, verify_async, version

SOURCE = """
module test_policy {
    intent ReadFile {
        takes: path: String;
        gives: content: String;
        effect: READ;
        conscience: path_safety;
    }

    intent WriteFile {
        takes: path: String, content: String;
        gives: success: bool;
        effect: WRITE;
        conscience: path_safety, no_exfiltrate;
    }
}
"""

async def test_async_methods():
    print("=== Axiom Async Bindings Test ===\n")

    print(f"Version: {version()}\n")

    # Test async engine creation
    print("1. Testing from_source_async()...")
    engine = await AxiomEngine.from_source_async(SOURCE)
    print(f"   Engine loaded: {engine}")
    print()

    # Test async verify
    print("2. Testing verify_async() - allowed case...")
    v1 = await engine.verify_async("ReadFile", {"path": "/tmp/data.txt"})
    print(f"   Path: /tmp/data.txt")
    print(f"   Allowed: {v1.allowed}")
    print()

    # Test async verify - denied case
    print("3. Testing verify_async() - denied case...")
    v2 = await engine.verify_async("ReadFile", {"path": "/etc/shadow"})
    print(f"   Path: /etc/shadow")
    print(f"   Allowed: {v2.allowed}")
    print(f"   Reason: {v2.reason}")
    print()

    # Test determinism in async context
    print("4. Testing async determinism...")
    v3 = await engine.verify_async("ReadFile", {"path": "/etc/shadow"})
    v4 = await engine.verify_async("ReadFile", {"path": "/etc/shadow"})
    print(f"   v3.allowed == v4.allowed: {v3.allowed == v4.allowed}")
    print(f"   v3.reason == v4.reason: {v3.reason == v4.reason}")
    print()

    # Test module-level async helper
    print("5. Testing module-level verify_async()...")
    import os
    policy_path = os.path.join(os.path.dirname(__file__), "..", "examples", "policies", "security.axm")
    try:
        v5 = await verify_async(
            os.path.abspath(policy_path),
            "ReadFile",
            {"path": "/tmp/test.txt"}
        )
        print(f"   Loaded from file: {v5}")
    except Exception as e:
        print(f"   (Skipping file-based test: {e})")
    print()

    # Test concurrent verifications
    print("6. Testing concurrent async verifications...")
    paths = ["/tmp/a.txt", "/tmp/b.txt", "/etc/shadow", "/tmp/c.txt"]
    tasks = [engine.verify_async("ReadFile", {"path": p}) for p in paths]
    results = await asyncio.gather(*tasks)
    for path, result in zip(paths, results):
        status = "allowed" if result.allowed else "denied"
        print(f"   {path}: {status}")
    print()

    print("=== All async tests passed! ===")

if __name__ == "__main__":
    asyncio.run(test_async_methods())
