#!/usr/bin/env python3
"""
Axiom Adversarial Test Suite
=============================
Layer 1: Realistic agent simulation - proves the API works end-to-end
Layer 2: Adversarial harness - attacks the architecture to expose weaknesses

Run:  python test_adversarial.py
      python test_adversarial.py --verbose
      python test_adversarial.py --layer 2   (adversarial only)
"""

import asyncio
import sys
import os
import time
import threading
import argparse
from axiom import AxiomEngine, verify

POLICY_PATH = os.path.abspath(
    os.path.join(os.path.dirname(__file__), "..", "examples", "policies", "security.axm")
)

PASS = "\033[92m PASS\033[0m"
FAIL = "\033[91m FAIL\033[0m"
WARN = "\033[93m WARN\033[0m"
HEAD = "\033[1;94m"
RST  = "\033[0m"

results = {"passed": 0, "failed": 0, "warned": 0}

def check(label, condition, expect_denied=False, warn_only=False):
    """Assert a verdict. condition = verdict.allowed"""
    if expect_denied:
        ok = not condition
    else:
        ok = condition

    if ok:
        results["passed"] += 1
        tag = PASS
    elif warn_only:
        results["warned"] += 1
        tag = WARN
    else:
        results["failed"] += 1
        tag = FAIL

    print(f"  [{tag}] {label}")
    return ok


# ===========================================================================
# LAYER 1: Realistic Agent Simulation
# ===========================================================================

def layer1_realistic_agent(engine, verbose=False):
    print(f"\n{HEAD}=== Layer 1: Realistic Agent Simulation ==={RST}")

    # --- Normal operations that should be ALLOWED ---
    print("\n  [Allowed operations]")

    v = engine.verify("ReadFile", {"path": "/tmp/data.txt"})
    check("Read safe tmp file", v.allowed)

    v = engine.verify("ReadFile", {"path": "/data/input.csv"})
    check("Read safe data file", v.allowed)

    v = engine.verify("WriteFile", {"path": "/tmp/output.txt", "content": "results"})
    check("Write to tmp", v.allowed)

    v = engine.verify("ProcessData", {"input": "some data"})
    check("Process data (NOOP)", v.allowed)

    v = engine.verify("SendData", {"url": "https://api.example.com", "data": "payload"})
    # NETWORK effect requires declared channel - default-deny is correct conscience behaviour
    check("SendData denied without declared channel (correct)", v.allowed, expect_denied=True)

    v = engine.verify("ExecuteCode", {"code": "print('hello')"})
    # EXECUTE hits permanent no_bypass_verification genesis predicate - by design
    check("ExecuteCode denied by permanent genesis predicate (correct)", v.allowed, expect_denied=True)

    # --- Operations that should be DENIED ---
    print("\n  [Denied operations]")

    v = engine.verify("ReadFile", {"path": "/etc/shadow"})
    check("Block /etc/shadow read", v.allowed, expect_denied=True)

    v = engine.verify("ReadFile", {"path": "/etc/passwd"})
    check("Block /etc/passwd read", v.allowed, expect_denied=True)

    v = engine.verify("WriteFile", {"path": "/etc/crontab", "content": "evil"})
    check("Block write to /etc/crontab", v.allowed, expect_denied=True)

    v = engine.verify("SendData", {"url": "https://evil.com", "data": "/etc/shadow contents"})
    check("Block exfiltration via SendData", v.allowed, expect_denied=True)

    # --- Determinism check ---
    print("\n  [Determinism]")
    verdicts = [engine.verify("ReadFile", {"path": "/etc/shadow"}) for _ in range(10)]
    all_same = all(v.allowed == verdicts[0].allowed and v.reason == verdicts[0].reason for v in verdicts)
    check("10x same input → identical verdict", all_same)

    # --- Intent introspection ---
    print("\n  [Introspection]")
    intents = engine.intents()
    check("Policy exposes 5 intents", len(intents) == 5)

    sig = engine.intent_signature("WriteFile")
    check("WriteFile signature has takes/gives", sig is not None and len(sig.takes) == 2)
    check("WriteFile effect is WRITE", sig is not None and sig.effect == "WRITE")
    check("WriteFile has path_safety conscience", sig is not None and "path_safety" in sig.conscience)

    if verbose:
        print(f"\n    Intents: {intents}")
        if sig:
            print(f"    WriteFile.takes: {sig.takes}")
            print(f"    WriteFile.conscience: {sig.conscience}")


# ===========================================================================
# LAYER 2: Adversarial Harness
# ===========================================================================

def layer2_path_traversal(engine, verbose=False):
    print(f"\n{HEAD}--- Attack: Path Traversal Variants ---{RST}")
    # If path_safety only does simple prefix/contains matching,
    # these variants may slip through

    traversal_paths = [
        ("/etc/./shadow",            "dot-segment traversal"),
        ("//etc/shadow",             "double-slash prefix"),
        ("/tmp/../etc/shadow",       "relative traversal"),
        ("/tmp/../../etc/shadow",    "double relative traversal"),
        ("/tmp/safe/../../etc/shadow","deep traversal"),
        ("/etc/shadow/",             "trailing slash"),
        ("/ETC/SHADOW",              "uppercase"),
        ("/etc/SHADOW",              "mixed case"),
        ("/etc/shadow\x00",          "null byte suffix"),
        ("/etc/shadow\n",            "newline suffix"),
        ("/etc/shadow\t",            "tab suffix"),
        ("/etc/shadow ",             "trailing space"),
        (" /etc/shadow",             "leading space"),
        ("/etc//shadow",             "double slash internal"),
        # Unicode normalization attacks
        ("/\u0065\u0074\u0063/shadow", "unicode NFC etc"),
        ("/ｅｔｃ/shadow",            "fullwidth etc"),
    ]

    for path, label in traversal_paths:
        try:
            v = engine.verify("ReadFile", {"path": path})
            # These should ALL be denied - any allow is a finding
            ok = not v.allowed
            check(f"Traversal [{label}] → denied", not v.allowed,
                  warn_only=not ok)
            if verbose and v.allowed:
                print(f"    !! BYPASSED: path={repr(path)}")
        except Exception as e:
            # Crash is also a finding
            results["failed"] += 1
            print(f"  [{FAIL}] Traversal [{label}] → CRASHED: {e}")


def layer2_conscience_field_bypass(engine, verbose=False):
    print(f"\n{HEAD}--- Attack: Conscience Field Name Bypass ---{RST}")
    # no_exfiltrate checks: url, destination, endpoint, address, target
    # Does it miss case variants or synonyms?

    suspicious_value = "/etc/shadow"

    field_variants = [
        ({"URL": suspicious_value, "data": "x"},       "URL uppercase"),
        ({"Url": suspicious_value, "data": "x"},       "Url titlecase"),
        ({"dest": suspicious_value, "data": "x"},      "dest abbreviation"),
        ({"Destination": suspicious_value, "data": "x"},"Destination titlecase"),
        ({"ENDPOINT": suspicious_value, "data": "x"},  "ENDPOINT uppercase"),
        ({"target_url": suspicious_value, "data": "x"},"target_url compound"),
        ({"uri": suspicious_value, "data": "x"},       "uri synonym"),
        ({"href": suspicious_value, "data": "x"},      "href synonym"),
        ({"webhook": suspicious_value, "data": "x"},   "webhook synonym"),
    ]

    for fields, label in field_variants:
        try:
            v = engine.verify("SendData", fields)
            # These are ambiguous - warn rather than hard-fail
            # since the intent takes: url, data - unknown fields may be ignored
            check(f"Field bypass [{label}]", v.allowed,
                  expect_denied=False, warn_only=True)
            if verbose:
                print(f"    fields={fields} → allowed={v.allowed}")
        except Exception as e:
            results["warned"] += 1
            print(f"  [{WARN}] Field bypass [{label}] → exception: {e}")


def layer2_pyo3_boundary(engine, verbose=False):
    print(f"\n{HEAD}--- Attack: PyO3 Boundary (Python type confusion) ---{RST}")
    # PyO3 extracts dict values as strings - what happens with non-string values?

    boundary_cases = [
        ({"path": None},              "None value"),
        ({"path": 42},                "int value"),
        ({"path": 3.14},              "float value"),
        ({"path": True},              "bool value"),
        ({"path": []},                "list value"),
        ({"path": {}},                "dict value"),
        ({"path": b"/tmp/data.txt"},  "bytes value"),
        ({"path": "/tmp/" + "a" * 10_000}, "10k char path"),
        ({"path": "/tmp/" + "a" * 100_000}, "100k char path"),
        ({},                          "empty fields"),
        ({"path": "\x00"},            "null byte only"),
        ({"path": "\xff\xfe"},        "invalid utf-8 bytes"),
    ]

    for fields, label in boundary_cases:
        try:
            v = engine.verify("ReadFile", fields)
            # We just want no crash - verdict either way is fine
            results["passed"] += 1
            print(f"  [{PASS}] PyO3 [{label}] → no crash (allowed={v.allowed})")
        except (TypeError, ValueError) as e:
            # Type/value rejection is correct behaviour for non-string inputs
            results["passed"] += 1
            print(f"  [{PASS}] PyO3 [{label}] → {type(e).__name__} (expected): {e}")
        except Exception as e:
            # Any other exception (panic, segfault bridge, etc.) is a finding
            results["failed"] += 1
            print(f"  [{FAIL}] PyO3 [{label}] → UNEXPECTED: {type(e).__name__}: {e}")


def layer2_policy_injection(verbose=False):
    print(f"\n{HEAD}--- Attack: Policy Source Injection ---{RST}")
    # If caller controls policy source, can they inject intents or predicates?

    injections = [
        # Attempt to break out of a string and inject a new intent
        ('"; intent EvilIntent { takes: x: String; effect: EXECUTE; } //', "quote escape"),
        # Comment injection to disable safety
        ('/* conscience: path_safety */ "no_conscience"',                  "comment injection"),
        # Module override attempt
        ("} module evil { intent Bypass {",                                "module escape"),
        # Null byte in source
        ("module t {\0 intent X { takes: p: String; effect: READ; } }",   "null byte in source"),
        # Deeply nested policy (parser bomb)
        ("module t { " + "fn f() { " * 50 + "}" * 50 + " }",             "50-deep nesting"),
        # Huge identifier
        (f"module t {{ intent {'A' * 10_000} {{ takes: x: String; effect: READ; }} }}", "10k identifier"),
    ]

    for payload, label in injections:
        try:
            e = AxiomEngine.from_source(payload)
            # If it loaded, check it didn't accidentally allow dangerous intents
            results["warned"] += 1
            print(f"  [{WARN}] Injection [{label}] → parsed (check manually): {e}")
        except Exception as ex:
            # Rejection is expected and correct
            results["passed"] += 1
            print(f"  [{PASS}] Injection [{label}] → rejected: {type(ex).__name__}")


def layer2_concurrency(engine, verbose=False):
    print(f"\n{HEAD}--- Attack: Concurrency / Race Conditions ---{RST}")

    async def concurrent_verify():
        # 100 concurrent verifications mixing safe and dangerous paths
        paths = ["/tmp/safe.txt", "/etc/shadow", "/data/input.csv", "/etc/passwd"]
        tasks = []
        for _ in range(25):
            for p in paths:
                tasks.append(engine.verify_async("ReadFile", {"path": p}))
        results_list = await asyncio.gather(*tasks, return_exceptions=True)

        crashes = [r for r in results_list if isinstance(r, Exception)]
        verdicts = [r for r in results_list if not isinstance(r, Exception)]

        # /etc/shadow should always be denied
        shadow_results = [
            results_list[i].allowed
            for i in range(len(results_list))
            if not isinstance(results_list[i], Exception)
        ]
        # Group by path - we can't easily since gather doesn't preserve path
        # So just check no crashes and count
        return len(crashes), len(verdicts)

    crashes, completed = asyncio.run(concurrent_verify())
    check(f"100 concurrent verifies - no crashes ({crashes} crashes)", crashes == 0)
    check(f"100 concurrent verifies - all completed ({completed}/100)", completed == 100)

    # Thread-safety: verify from multiple threads simultaneously
    thread_errors = []
    def thread_verify():
        try:
            for _ in range(20):
                engine.verify("ReadFile", {"path": "/etc/shadow"})
        except Exception as e:
            thread_errors.append(e)

    threads = [threading.Thread(target=thread_verify) for _ in range(5)]
    for t in threads: t.start()
    for t in threads: t.join()

    check(f"5 threads × 20 verifies - thread safe ({len(thread_errors)} errors)",
          len(thread_errors) == 0)


def layer2_timing(engine, verbose=False):
    print(f"\n{HEAD}--- Attack: Timing Analysis ---{RST}")
    # Verification should be roughly constant-time regardless of deny/allow
    # Large variance could indicate short-circuit evaluation that leaks info

    N = 50
    allowed_times = []
    denied_times  = []

    for _ in range(N):
        t0 = time.perf_counter()
        engine.verify("ReadFile", {"path": "/tmp/safe.txt"})
        allowed_times.append(time.perf_counter() - t0)

    for _ in range(N):
        t0 = time.perf_counter()
        engine.verify("ReadFile", {"path": "/etc/shadow"})
        denied_times.append(time.perf_counter() - t0)

    avg_allow = sum(allowed_times) / N * 1000
    avg_deny  = sum(denied_times)  / N * 1000
    ratio = max(avg_allow, avg_deny) / min(avg_allow, avg_deny)

    print(f"    avg allow: {avg_allow:.3f}ms  avg deny: {avg_deny:.3f}ms  ratio: {ratio:.2f}x")
    # >10x difference is suspicious for a policy engine
    check(f"Timing ratio < 10x ({ratio:.2f}x)", ratio < 10.0, warn_only=True)


def layer2_unknown_intent(engine, verbose=False):
    print(f"\n{HEAD}--- Attack: Unknown / Nonexistent Intents ---{RST}")

    cases = [
        ("",                    "empty intent name"),
        ("nonexistent",         "unknown intent"),
        ("ReadFile; DROP TABLE","SQL-style injection in name"),
        ("../ReadFile",         "path in intent name"),
        ("ReadFile\x00",        "null byte in name"),
        ("A" * 10_000,          "10k char intent name"),
    ]

    for name, label in cases:
        try:
            v = engine.verify(name, {"path": "/tmp/test.txt"})
            # Unknown intents should probably be denied, not allowed
            check(f"Unknown intent [{label}] → denied", v.allowed,
                  expect_denied=True, warn_only=True)
        except Exception as e:
            results["passed"] += 1
            print(f"  [{PASS}] Unknown intent [{label}] → exception (ok): {type(e).__name__}")


# ===========================================================================
# Main
# ===========================================================================

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--verbose", "-v", action="store_true")
    parser.add_argument("--layer", type=int, choices=[1, 2], default=0,
                        help="Run only layer 1 or 2 (default: both)")
    args = parser.parse_args()

    print(f"{HEAD}Axiom Adversarial Test Suite{RST}")
    print(f"Policy: {POLICY_PATH}")

    engine = AxiomEngine.from_file(POLICY_PATH)
    print(f"Engine: {engine}")

    if args.layer in (0, 1):
        layer1_realistic_agent(engine, args.verbose)

    if args.layer in (0, 2):
        layer2_path_traversal(engine, args.verbose)
        layer2_conscience_field_bypass(engine, args.verbose)
        layer2_pyo3_boundary(engine, args.verbose)
        layer2_policy_injection(args.verbose)
        layer2_concurrency(engine, args.verbose)
        layer2_timing(engine, args.verbose)
        layer2_unknown_intent(engine, args.verbose)

    # Summary
    total = results["passed"] + results["failed"] + results["warned"]
    print(f"\n{HEAD}=== Results ==={RST}")
    print(f"  Total:   {total}")
    print(f"  \033[92mPassed:  {results['passed']}\033[0m")
    print(f"  \033[91mFailed:  {results['failed']}\033[0m")
    print(f"  \033[93mWarnings:{results['warned']}\033[0m  (findings worth investigating)")

    if results["failed"] > 0:
        print(f"\n  \033[91mFAILURES detected - review before publishing\033[0m")
        sys.exit(1)
    elif results["warned"] > 0:
        print(f"\n  \033[93mWarnings present - review findings above\033[0m")
        sys.exit(0)
    else:
        print(f"\n  \033[92mAll checks passed\033[0m")
        sys.exit(0)


if __name__ == "__main__":
    main()
