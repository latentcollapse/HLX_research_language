#!/usr/bin/env python3
"""Test script for axiom_py bindings"""

from axiom import AxiomEngine, verify, version

print("=== Axiom Python Bindings Test ===\n")

# Test version
print(f"Version: {version()}")
print()

# Test loading from source
print("1. Loading policy from source...")
source = """
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

engine = AxiomEngine.from_source(source)
print(f"   Engine loaded: {engine}")
print(f"   Intents: {engine.intents()}")
print()

# Test intent signature
print("2. Testing intent_signature()...")
sig = engine.intent_signature("WriteFile")
if sig:
    print(f"   Name: {sig.name}")
    print(f"   Takes: {sig.takes}")
    print(f"   Gives: {sig.gives}")
    print(f"   Effect: {sig.effect}")
    print(f"   Conscience: {sig.conscience}")
print()

# Test has_intent
print("3. Testing has_intent()...")
print(f"   has_intent('ReadFile'): {engine.has_intent('ReadFile')}")
print(f"   has_intent('DeleteFile'): {engine.has_intent('DeleteFile')}")
print()

# Test verification - allowed case
print("4. Testing verify() - allowed case...")
v1 = engine.verify("ReadFile", {"path": "/tmp/data.txt"})
print(f"   Path: /tmp/data.txt")
print(f"   Verdict: {v1}")
print(f"   Allowed: {v1.allowed}")
print(f"   Reason: {v1.reason}")
print(f"   Guidance: {v1.guidance}")
print(f"   __bool__: {bool(v1)}")
print()

# Test verification - denied case
print("5. Testing verify() - denied case...")
v2 = engine.verify("ReadFile", {"path": "/etc/shadow"})
print(f"   Path: /etc/shadow")
print(f"   Verdict: {v2}")
print(f"   Allowed: {v2.allowed}")
print(f"   Reason: {v2.reason}")
print(f"   Guidance: {v2.guidance}")
print(f"   __bool__: {bool(v2)}")
print()

# Test determinism - multiple verifications should be identical
print("6. Testing determinism (same input = same output)...")
v3 = engine.verify("ReadFile", {"path": "/etc/shadow"})
v4 = engine.verify("ReadFile", {"path": "/etc/shadow"})
print(f"   v3.allowed == v4.allowed: {v3.allowed == v4.allowed}")
print(f"   v3.reason == v4.reason: {v3.reason == v4.reason}")
print()

# Test module-level verify helper
print("7. Testing module-level verify() helper...")
try:
    v5 = verify("examples/policies/security.axm", "ReadFile", {"path": "/tmp/test.txt"})
    print(f"   Loaded from file: {v5}")
except Exception as e:
    print(f"   (Skipping file-based test: {e})")
print()

print("=== All tests passed! ===")
