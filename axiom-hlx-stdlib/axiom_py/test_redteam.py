#!/usr/bin/env python3
"""
Axiom Red Team Test Suite
=========================
Comprehensive adversarial testing against the Axiom policy engine.
Designed to run inside the axiom-redteam container with radamsa available.

Layers:
  1. Functional baseline     — verify the engine works correctly
  2. Path traversal & escape — filesystem conscience bypass attempts
  3. Unicode & encoding       — normalization, homoglyph, and encoding attacks
  4. Parser fuzzing           — radamsa-mutated .axm source fed to from_source()
  5. PyO3 boundary            — type confusion, memory pressure, null bytes
  6. Concurrency stress       — race conditions, thread safety, async safety
  7. Timing oracle            — statistical timing analysis for info leaks
  8. Policy injection         — attempt to inject intents, escape modules, override conscience
  9. Python layer attacks     — target builder, presets, guard, integrations
 10. Preset escape            — attempt to bypass GuardedEngine allow-list

Run:
  PYTHONPATH=python python3 test_redteam.py
  PYTHONPATH=python python3 test_redteam.py --verbose
  PYTHONPATH=python python3 test_redteam.py --layer 4   (fuzzing only)
"""

from __future__ import annotations

import argparse
import asyncio
import functools
import gc
import json
import os
import resource
import signal
import subprocess
import sys
import textwrap
import threading
import time
import traceback
from typing import Any, Callable, Dict, List, Optional, Tuple

# ---------------------------------------------------------------------------
# Axiom imports
# ---------------------------------------------------------------------------
from axiom import AxiomEngine, PolicyBuilder, Effect, Conscience, guard, AxiomDenied
from axiom.presets import (
    filesystem_readonly,
    filesystem_readwrite,
    network_egress,
    code_execution_sandboxed,
    agent_standard,
    coding_assistant,
    GuardedEngine,
    _SyntheticDenial,
)
from axiom.integrations.langchain import AxiomGuardedTool
from axiom.integrations.openai import AxiomInterceptor
from axiom.guard import _to_pascal_case, _default_conscience_for_effect

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------
POLICY_PATH = os.path.abspath(
    os.path.join(os.path.dirname(__file__), "..", "examples", "policies", "security.axm")
)
AXIOM_BIN = "/tmp/axiom_build/axiom/target/release/axiom"
RADAMSA = "radamsa"

# ---------------------------------------------------------------------------
# Terminal colors
# ---------------------------------------------------------------------------
PASS = "\033[92m PASS\033[0m"
FAIL = "\033[91m FAIL\033[0m"
WARN = "\033[93m WARN\033[0m"
CRIT = "\033[91;1m CRIT\033[0m"
INFO = "\033[96m INFO\033[0m"
HEAD = "\033[1;94m"
RST = "\033[0m"
BOLD = "\033[1m"

# ---------------------------------------------------------------------------
# Result tracking
# ---------------------------------------------------------------------------
results = {"passed": 0, "failed": 0, "warned": 0, "critical": 0}
findings: List[str] = []


def check(label: str, condition: bool, *, expect_denied: bool = False,
          warn_only: bool = False, critical: bool = False) -> bool:
    if expect_denied:
        ok = not condition
    else:
        ok = condition

    if ok:
        results["passed"] += 1
        tag = PASS
    elif critical:
        results["critical"] += 1
        tag = CRIT
        findings.append(f"[CRITICAL] {label}")
    elif warn_only:
        results["warned"] += 1
        tag = WARN
        findings.append(f"[WARN] {label}")
    else:
        results["failed"] += 1
        tag = FAIL
        findings.append(f"[FAIL] {label}")

    print(f"  [{tag}] {label}")
    return ok


def section(title: str):
    print(f"\n{HEAD}{'=' * 60}{RST}")
    print(f"{HEAD}  {title}{RST}")
    print(f"{HEAD}{'=' * 60}{RST}")


def subsection(title: str):
    print(f"\n{HEAD}--- {title} ---{RST}")


# ---------------------------------------------------------------------------
# Utility: timeout decorator for individual tests
# ---------------------------------------------------------------------------
class TestTimeout(Exception):
    pass


def with_timeout(seconds: int = 5):
    def decorator(func):
        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            def handler(signum, frame):
                raise TestTimeout(f"{func.__name__} timed out after {seconds}s")
            old = signal.signal(signal.SIGALRM, handler)
            signal.alarm(seconds)
            try:
                return func(*args, **kwargs)
            finally:
                signal.alarm(0)
                signal.signal(signal.SIGALRM, old)
        return wrapper
    return decorator


# ===========================================================================
# LAYER 1: Functional Baseline
# ===========================================================================

def layer1_baseline(engine: AxiomEngine, verbose: bool = False):
    section("Layer 1: Functional Baseline")

    subsection("Allowed operations")
    for path, label in [
        ("/tmp/data.txt", "Read /tmp file"),
        ("/data/input.csv", "Read /data file"),
        ("/home/user/doc.txt", "Read /home file"),
        ("/var/log/app.log", "Read /var/log file"),
    ]:
        v = engine.verify("ReadFile", {"path": path})
        check(label, v.allowed)

    v = engine.verify("WriteFile", {"path": "/tmp/out.txt", "content": "data"})
    check("Write to /tmp", v.allowed)

    v = engine.verify("ProcessData", {"input": "data"})
    check("ProcessData (NOOP effect)", v.allowed)

    subsection("Denied operations")
    for path, label in [
        ("/etc/shadow", "Block /etc/shadow"),
        ("/etc/passwd", "Block /etc/passwd"),
        ("/proc/1/environ", "Block /proc access"),
        ("/sys/kernel/notes", "Block /sys access"),
        ("/boot/vmlinuz", "Block /boot access"),
        ("/root/.ssh/id_rsa", "Block /root access"),
        ("/dev/sda", "Block /dev access"),
    ]:
        v = engine.verify("ReadFile", {"path": path})
        check(label, v.allowed, expect_denied=True)

    subsection("Determinism (G3)")
    verdicts_deny = [engine.verify("ReadFile", {"path": "/etc/shadow"}) for _ in range(100)]
    all_same = all(
        v.allowed == verdicts_deny[0].allowed and v.reason == verdicts_deny[0].reason
        for v in verdicts_deny
    )
    check("100x identical deny verdict", all_same, critical=True)

    verdicts_allow = [engine.verify("ReadFile", {"path": "/tmp/safe.txt"}) for _ in range(100)]
    all_same = all(v.allowed == verdicts_allow[0].allowed for v in verdicts_allow)
    check("100x identical allow verdict", all_same, critical=True)

    subsection("Specific denial (G5)")
    v = engine.verify("ReadFile", {"path": "/etc/shadow"})
    check("Denied verdict has .reason", hasattr(v, "reason") and v.reason is not None)
    check("Denied verdict has .guidance", hasattr(v, "guidance") and v.guidance is not None)
    check("Denial cites path_safety", "path_safety" in (v.reason or ""), critical=True)
    check("Denial cites the path value", "shadow" in (v.reason or ""), critical=True)


# ===========================================================================
# LAYER 2: Path Traversal & Escape
# ===========================================================================

def layer2_path_traversal(engine: AxiomEngine, verbose: bool = False):
    section("Layer 2: Path Traversal & Escape")

    traversals = [
        # Basic traversal
        ("/etc/./shadow", "dot-segment /etc/./shadow"),
        ("//etc/shadow", "double-slash prefix"),
        ("/tmp/../etc/shadow", "relative traversal"),
        ("/tmp/../../etc/shadow", "double relative"),
        ("/tmp/safe/../../etc/shadow", "deep relative"),
        ("/./etc/shadow", "dot-prefix traversal"),

        # Trailing characters
        ("/etc/shadow/", "trailing slash"),
        ("/etc/shadow/.", "trailing dot"),
        ("/etc/shadow/..", "trailing dotdot"),
        ("/etc/shadow\x00", "null byte suffix"),
        ("/etc/shadow\x00safe.txt", "null byte mid-path"),
        ("/etc/shadow\n", "newline suffix"),
        ("/etc/shadow\r", "carriage return suffix"),
        ("/etc/shadow\t", "tab suffix"),
        ("/etc/shadow ", "trailing space"),
        (" /etc/shadow", "leading space"),

        # Case manipulation
        ("/ETC/SHADOW", "uppercase"),
        ("/Etc/Shadow", "titlecase"),
        ("/etc/SHADOW", "mixed case"),

        # Double encoding simulation (string-level, not URL-decoded)
        ("/etc%2Fshadow", "percent-encoded slash"),
        ("/%65%74%63/shadow", "percent-encoded etc"),
        ("/etc/shad%6fw", "percent-encoded char"),

        # Symlink-like patterns
        ("/tmp/link -> /etc/shadow", "arrow notation"),
        ("/proc/self/root/etc/shadow", "procfs root escape"),
        ("/proc/1/root/etc/shadow", "procfs pid root escape"),

        # Multiple slashes
        ("/etc//shadow", "double slash internal"),
        ("/etc///shadow", "triple slash"),
        ("///etc///shadow///", "excessive slashes"),

        # Backslash (Windows-style on Linux)
        ("/etc\\shadow", "backslash separator"),
        ("\\etc\\shadow", "full backslash path"),
    ]

    for path, label in traversals:
        try:
            v = engine.verify("ReadFile", {"path": path})
            ok = not v.allowed
            check(f"Traversal: {label}", v.allowed, expect_denied=True,
                  warn_only=not ok, critical=False)
            if verbose and v.allowed:
                print(f"    !! BYPASSED: {repr(path)}")
        except Exception as e:
            results["failed"] += 1
            findings.append(f"[CRASH] Traversal {label}: {e}")
            print(f"  [{FAIL}] Traversal: {label} → CRASHED: {type(e).__name__}: {e}")


# ===========================================================================
# LAYER 3: Unicode & Encoding Attacks
# ===========================================================================

def layer3_unicode(engine: AxiomEngine, verbose: bool = False):
    section("Layer 3: Unicode & Encoding Attacks")

    subsection("Homoglyph attacks")
    homoglyphs = [
        # Cyrillic lookalikes
        ("/\u0435tc/shadow", "Cyrillic е in etc"),
        ("/e\u0442c/shadow", "Cyrillic т in etc"),
        ("/et\u0441/shadow", "Cyrillic с in etc"),
        ("/\u0435\u0442\u0441/shadow", "Full Cyrillic етс"),

        # Fullwidth Latin
        ("/\uff45\uff54\uff43/shadow", "Fullwidth etc"),
        ("/etc/\uff53hadow", "Fullwidth s in shadow"),

        # Mathematical variants
        ("/\U0001d41e\U0001d42d\U0001d41c/shadow", "Math bold etc"),

        # Combining characters
        ("/etc\u0300/shadow", "Combining grave on c"),
        ("/etc\u200b/shadow", "Zero-width space after etc"),
        ("/etc\u200d/shadow", "Zero-width joiner after etc"),
        ("/etc\ufeff/shadow", "BOM after etc"),
        ("/etc\u00ad/shadow", "Soft hyphen after etc"),

        # Right-to-left override
        ("/etc/\u202eshadow", "RTL override in filename"),
        ("/\u202eetc/shadow", "RTL override in directory"),

        # Confusable path separators
        ("/etc\u2215shadow", "Division slash ∕"),
        ("/etc\u2044shadow", "Fraction slash ⁄"),
        ("/etc\uff0fshadow", "Fullwidth solidus ／"),
    ]

    for path, label in homoglyphs:
        try:
            v = engine.verify("ReadFile", {"path": path})
            # Most of these should be denied out of caution, but some may
            # legitimately resolve to different paths on the filesystem
            check(f"Homoglyph: {label}", v.allowed, expect_denied=True,
                  warn_only=True)
        except Exception as e:
            results["passed"] += 1
            print(f"  [{PASS}] Homoglyph: {label} → rejected: {type(e).__name__}")

    subsection("NFC/NFD normalization")
    # é as single codepoint vs decomposed
    nfc = "/tmp/caf\u00e9.txt"  # single codepoint
    nfd = "/tmp/cafe\u0301.txt"  # e + combining acute
    v1 = engine.verify("ReadFile", {"path": nfc})
    v2 = engine.verify("ReadFile", {"path": nfd})
    # Both should produce the same verdict (both are safe paths)
    check("NFC and NFD produce consistent verdicts", v1.allowed == v2.allowed)

    subsection("Overlong UTF-8 simulation")
    # Python strings are Unicode, but test what happens with unusual chars
    weird_paths = [
        ("\x00/etc/shadow", "leading null byte"),
        ("/etc/shadow\x7f", "DEL character"),
        ("/etc/shadow" + "\x80" * 10, "high bytes"),
        ("/" + "\U0001f4a9" * 100 + "/etc/shadow", "emoji-padded path"),
    ]
    for path, label in weird_paths:
        try:
            v = engine.verify("ReadFile", {"path": path})
            check(f"Weird encoding: {label}", True)  # no crash = pass
        except (TypeError, ValueError):
            results["passed"] += 1
            print(f"  [{PASS}] Weird encoding: {label} → type rejection (ok)")
        except Exception as e:
            results["failed"] += 1
            print(f"  [{FAIL}] Weird encoding: {label} → {type(e).__name__}: {e}")


# ===========================================================================
# LAYER 4: Parser Fuzzing (radamsa)
# ===========================================================================

def layer4_parser_fuzz(verbose: bool = False):
    section("Layer 4: Parser Fuzzing (radamsa)")

    # Seed corpus: valid .axm policies
    seeds = [
        'module t { intent R { takes: p: String; effect: READ; conscience: path_safety; } }',
        'module t { intent W { takes: p: String, c: String; effect: WRITE; conscience: path_safety, no_exfiltrate; } }',
        'module t { intent E { takes: c: String; effect: EXECUTE; conscience: no_harm, no_bypass_verification; } }',
        'module t { intent N { takes: u: String; effect: NETWORK; conscience: no_exfiltrate; } }',
        'module t { intent P { effect: NOOP; } }',
    ]

    # Check if radamsa is available
    try:
        subprocess.run([RADAMSA, "--help"], capture_output=True, timeout=5)
    except (FileNotFoundError, subprocess.TimeoutExpired):
        print(f"  [{INFO}] radamsa not found — skipping mutation fuzzing")
        return

    total_mutations = 0
    crashes = 0
    hangs = 0
    accepted = 0

    subsection(f"Mutation fuzzing ({len(seeds)} seeds × 100 mutations)")

    for seed_idx, seed in enumerate(seeds):
        try:
            proc = subprocess.run(
                [RADAMSA, "-n", "100"],
                input=seed.encode("utf-8", errors="replace"),
                capture_output=True,
                timeout=10,
            )
            mutations = proc.stdout.split(b"\n")
        except Exception as e:
            print(f"  [{WARN}] radamsa failed on seed {seed_idx}: {e}")
            continue

        for mutation in mutations:
            if not mutation:
                continue
            total_mutations += 1
            try:
                payload = mutation.decode("utf-8", errors="replace")
            except Exception:
                continue

            try:
                eng = AxiomEngine.from_source(payload)
                accepted += 1
                # If it parsed, try to verify something with it
                try:
                    for intent_name in eng.intents():
                        eng.verify(intent_name, {"path": "/etc/shadow", "p": "/etc/shadow"})
                except Exception:
                    pass  # Runtime errors on mutated policies are expected
            except (ValueError, RuntimeError):
                pass  # Parse rejection is expected and correct
            except MemoryError:
                crashes += 1
                findings.append(f"[CRASH] MemoryError on mutated input (seed {seed_idx})")
            except Exception as e:
                # Unexpected exception type = potential issue
                crashes += 1
                if verbose:
                    print(f"    Unexpected: {type(e).__name__}: {str(e)[:100]}")

    check(f"Fuzzed {total_mutations} mutations, {crashes} crashes", crashes == 0, critical=True)
    check(f"{accepted} mutations parsed (informational)", True)
    print(f"    Stats: {total_mutations} tested, {accepted} parsed, {crashes} crashes")

    subsection("Targeted parser bombs")
    bombs = [
        # Deeply nested braces
        ("module t { " + "{ " * 500 + "} " * 500 + " }", "500-deep nesting"),
        # Enormous identifier
        (f"module t {{ intent {'A' * 100_000} {{ effect: READ; }} }}", "100k identifier"),
        # Many intents
        ("module t { " + " ".join(
            f"intent I{i} {{ effect: READ; }}" for i in range(1000)
        ) + " }", "1000 intents"),
        # Repeated conscience predicates
        ("module t { intent R { effect: READ; conscience: " +
         ", ".join(["path_safety"] * 500) + "; } }", "500 repeated predicates"),
        # Very long string-like content
        (f'module t {{ intent R {{ takes: p: String; effect: READ; pre: length("{("x" * 100_000)}") > 0; }} }}',
         "100k string in pre clause"),
        # Empty module
        ("module t { }", "empty module"),
        # Multiple modules
        ("module a { intent X { effect: READ; } } module b { intent Y { effect: WRITE; } }",
         "multiple modules"),
        # Unicode in identifiers
        ("module t { intent Rëad { effect: READ; } }", "diacritic in intent name"),
        # Null bytes scattered
        ("module\x00t { intent\x00R { effect:\x00READ; } }", "null bytes throughout"),
    ]

    for payload, label in bombs:
        try:
            eng = AxiomEngine.from_source(payload)
            check(f"Parser bomb: {label} → parsed", True, warn_only=True)
        except (ValueError, RuntimeError):
            results["passed"] += 1
            print(f"  [{PASS}] Parser bomb: {label} → rejected (correct)")
        except MemoryError:
            results["critical"] += 1
            findings.append(f"[CRITICAL] MemoryError: {label}")
            print(f"  [{CRIT}] Parser bomb: {label} → MemoryError!")
        except Exception as e:
            results["failed"] += 1
            findings.append(f"[FAIL] Parser bomb: {label}: {type(e).__name__}")
            print(f"  [{FAIL}] Parser bomb: {label} → {type(e).__name__}: {str(e)[:80]}")


# ===========================================================================
# LAYER 5: PyO3 Boundary Attacks
# ===========================================================================

def layer5_pyo3(engine: AxiomEngine, verbose: bool = False):
    section("Layer 5: PyO3 Boundary Attacks")

    subsection("Type confusion")
    type_attacks = [
        ({"path": None}, "None value"),
        ({"path": 42}, "int value"),
        ({"path": 3.14}, "float value"),
        ({"path": True}, "bool True"),
        ({"path": False}, "bool False"),
        ({"path": []}, "empty list"),
        ({"path": ["/etc/shadow"]}, "list with path"),
        ({"path": {}}, "empty dict"),
        ({"path": {"nested": "value"}}, "nested dict"),
        ({"path": b"/etc/shadow"}, "bytes"),
        ({"path": object()}, "raw object"),
        ({"path": lambda: "/etc/shadow"}, "lambda"),
        ({"path": type}, "type object"),
    ]

    for fields, label in type_attacks:
        try:
            v = engine.verify("ReadFile", fields)
            # If it didn't crash but allowed, that's concerning for non-string types
            if v.allowed and label != "bool False":
                check(f"Type confusion: {label} → allowed (suspicious)", False, warn_only=True)
            else:
                check(f"Type confusion: {label} → no crash", True)
        except (TypeError, ValueError):
            results["passed"] += 1
            print(f"  [{PASS}] Type confusion: {label} → rejected (correct)")
        except Exception as e:
            results["failed"] += 1
            print(f"  [{FAIL}] Type confusion: {label} → {type(e).__name__}: {e}")

    subsection("Memory pressure")
    pressure_cases = [
        ({"path": "A" * 1_000_000}, "1MB path string"),
        ({"path": "/tmp/" + "a/" * 50_000}, "50k nested dirs"),
        ({f"field_{i}": f"value_{i}" for i in range(10_000)}, "10k fields"),
        ({"path": "\x00" * 100_000}, "100k null bytes"),
    ]

    for fields, label in pressure_cases:
        try:
            v = engine.verify("ReadFile", fields)
            check(f"Memory pressure: {label} → no crash", True)
        except (TypeError, ValueError, RuntimeError):
            results["passed"] += 1
            print(f"  [{PASS}] Memory pressure: {label} → rejected (ok)")
        except MemoryError:
            results["critical"] += 1
            findings.append(f"[CRITICAL] MemoryError: {label}")
            print(f"  [{CRIT}] Memory pressure: {label} → MemoryError!")
        except Exception as e:
            results["failed"] += 1
            print(f"  [{FAIL}] Memory pressure: {label} → {type(e).__name__}: {e}")

    subsection("Intent name attacks")
    name_attacks = [
        ("", "empty string"),
        ("\x00", "null byte"),
        ("A" * 1_000_000, "1M char name"),
        ("ReadFile\x00WriteFile", "null byte spliced intent"),
        ("ReadFile\nWriteFile", "newline spliced intent"),
        ("__import__('os').system('id')", "Python injection in name"),
        ("${PATH}", "shell variable expansion"),
        ("<script>alert(1)</script>", "XSS payload in name"),
        ("ReadFile; DROP TABLE", "SQL injection in name"),
        ("ReadFile' OR '1'='1", "SQL injection variant"),
    ]

    for name, label in name_attacks:
        try:
            v = engine.verify(name, {"path": "/tmp/safe.txt"})
            check(f"Intent name: {label} → handled", True, warn_only=True)
        except (ValueError, RuntimeError):
            results["passed"] += 1
            print(f"  [{PASS}] Intent name: {label} → rejected (correct)")
        except Exception as e:
            results["failed"] += 1
            print(f"  [{FAIL}] Intent name: {label} → {type(e).__name__}: {e}")


# ===========================================================================
# LAYER 6: Concurrency Stress
# ===========================================================================

def layer6_concurrency(engine: AxiomEngine, verbose: bool = False):
    section("Layer 6: Concurrency Stress")

    subsection("Async stress (500 concurrent)")

    async def async_stress():
        paths = ["/tmp/safe.txt", "/etc/shadow", "/data/input.csv",
                 "/etc/passwd", "/proc/self/environ", "/home/user/file.txt"]
        tasks = []
        for _ in range(500):
            p = paths[len(tasks) % len(paths)]
            tasks.append(engine.verify_async("ReadFile", {"path": p}))
        results_list = await asyncio.gather(*tasks, return_exceptions=True)
        crashes = [r for r in results_list if isinstance(r, Exception) and not isinstance(r, (RuntimeError,))]
        return len(crashes), len(results_list)

    crashes, total = asyncio.run(async_stress())
    check(f"500 async verifies — {crashes} unexpected exceptions", crashes == 0, critical=True)

    subsection("Thread stress (10 threads × 100 verifies)")

    thread_errors = []
    thread_results: Dict[int, List[bool]] = {}

    def thread_worker(tid: int):
        thread_results[tid] = []
        try:
            for _ in range(100):
                v = engine.verify("ReadFile", {"path": "/etc/shadow"})
                thread_results[tid].append(v.allowed)
        except Exception as e:
            thread_errors.append((tid, e))

    threads = [threading.Thread(target=thread_worker, args=(i,)) for i in range(10)]
    for t in threads:
        t.start()
    for t in threads:
        t.join(timeout=30)

    check(f"10 threads × 100 verifies — {len(thread_errors)} errors",
          len(thread_errors) == 0, critical=True)

    # Verify all threads got consistent results (all denied)
    all_denied = all(
        all(not allowed for allowed in results_list)
        for results_list in thread_results.values()
    )
    check("All threads agree: /etc/shadow always denied", all_denied, critical=True)

    subsection("Mixed read/write concurrency")

    async def mixed_concurrency():
        tasks = []
        for i in range(200):
            if i % 3 == 0:
                tasks.append(engine.verify_async("ReadFile", {"path": "/etc/shadow"}))
            elif i % 3 == 1:
                tasks.append(engine.verify_async("WriteFile", {"path": "/tmp/out.txt", "content": "x"}))
            else:
                tasks.append(engine.verify_async("ReadFile", {"path": "/tmp/safe.txt"}))
        return await asyncio.gather(*tasks, return_exceptions=True)

    mixed = asyncio.run(mixed_concurrency())
    crashes = sum(1 for r in mixed if isinstance(r, Exception) and not isinstance(r, RuntimeError))
    check(f"200 mixed intent async — {crashes} crashes", crashes == 0, critical=True)


# ===========================================================================
# LAYER 7: Timing Oracle
# ===========================================================================

def layer7_timing(engine: AxiomEngine, verbose: bool = False):
    section("Layer 7: Timing Oracle Analysis")

    N = 200  # Enough samples for statistical significance

    # Warm up JIT/cache
    for _ in range(50):
        engine.verify("ReadFile", {"path": "/tmp/safe.txt"})
        engine.verify("ReadFile", {"path": "/etc/shadow"})

    allow_times = []
    deny_times = []
    for _ in range(N):
        t0 = time.perf_counter_ns()
        engine.verify("ReadFile", {"path": "/tmp/safe.txt"})
        allow_times.append(time.perf_counter_ns() - t0)

    for _ in range(N):
        t0 = time.perf_counter_ns()
        engine.verify("ReadFile", {"path": "/etc/shadow"})
        deny_times.append(time.perf_counter_ns() - t0)

    avg_allow = sum(allow_times) / N / 1_000  # microseconds
    avg_deny = sum(deny_times) / N / 1_000
    ratio = max(avg_allow, avg_deny) / max(min(avg_allow, avg_deny), 0.001)

    # Standard deviation
    import math
    std_allow = math.sqrt(sum((t / 1000 - avg_allow) ** 2 for t in allow_times) / N)
    std_deny = math.sqrt(sum((t / 1000 - avg_deny) ** 2 for t in deny_times) / N)

    print(f"    Allow: {avg_allow:.1f}μs ± {std_allow:.1f}μs")
    print(f"    Deny:  {avg_deny:.1f}μs ± {std_deny:.1f}μs")
    print(f"    Ratio: {ratio:.2f}x")

    check(f"Timing ratio < 5x ({ratio:.2f}x)", ratio < 5.0, warn_only=True)
    check(f"Timing ratio < 10x ({ratio:.2f}x)", ratio < 10.0, critical=True)

    subsection("Path-length timing analysis")
    # Check if longer paths take proportionally longer (potential DoS)
    length_times = {}
    for length in [10, 100, 1000, 10000]:
        path = "/tmp/" + "a" * length
        times = []
        for _ in range(50):
            t0 = time.perf_counter_ns()
            engine.verify("ReadFile", {"path": path})
            times.append(time.perf_counter_ns() - t0)
        length_times[length] = sum(times) / 50 / 1000  # μs

    # Check if 10k path takes more than 100x of 10-char path
    if length_times[10] > 0:
        growth = length_times[10000] / length_times[10]
        print(f"    10-char: {length_times[10]:.1f}μs → 10k-char: {length_times[10000]:.1f}μs ({growth:.1f}x)")
        check(f"Path length scaling < 100x ({growth:.1f}x)", growth < 100.0, warn_only=True)


# ===========================================================================
# LAYER 8: Policy Injection
# ===========================================================================

def layer8_injection(verbose: bool = False):
    section("Layer 8: Policy Injection")

    subsection("Source injection attacks")
    injections = [
        # Escape attempts
        ('module t { intent R { effect: READ; } } module evil { intent Bypass { effect: NOOP; } }',
         "dual module injection"),
        ('module t { intent R { effect: READ; conscience: path_safety; } intent Secret { effect: NOOP; } }',
         "hidden NOOP intent"),
        ('"; intent Evil { effect: EXECUTE; } //', "quote escape"),
        ('/* conscience: path_safety */ module t { intent R { effect: READ; } }',
         "comment to strip conscience"),
        ('module t {\0 intent X { effect: READ; } }', "null byte in source"),

        # Effect manipulation
        ('module t { intent R { effect: NOOP; conscience: path_safety; } }',
         "downgrade READ to NOOP"),

        # Conscience stripping
        ('module t { intent R { effect: READ; } }',
         "intent without conscience"),
        ('module t { intent W { effect: WRITE; } }',
         "WRITE without any conscience"),
    ]

    for payload, label in injections:
        try:
            eng = AxiomEngine.from_source(payload)
            intents = eng.intents()

            # For the "hidden NOOP intent" test: check if an attacker-injected
            # intent with no safety conscience can bypass path checks.
            # This is expected when attacker controls policy source —
            # the mitigation is that policy source is a trust boundary.
            if "Secret" in intents:
                v = eng.verify("Secret", {"path": "/etc/shadow"})
                if v.allowed:
                    check(f"Injection: {label} → NOOP intent allows /etc/shadow (policy source = trust boundary)",
                          True, warn_only=True)
                else:
                    check(f"Injection: {label} → NOOP intent correctly denied", True)
                continue

            if "Evil" in intents or "Bypass" in intents:
                check(f"Injection: {label} → injected intent found!", False, critical=True)
                continue

            # Check if the parsed policy is actually safe
            for iname in intents:
                try:
                    v = eng.verify(iname, {"path": "/etc/shadow"})
                    if v.allowed and label in ("downgrade READ to NOOP", "intent without conscience",
                                                "WRITE without any conscience"):
                        check(f"Injection: {label} → allows /etc/shadow (effect downgrade)", False, warn_only=True)
                    else:
                        check(f"Injection: {label} → handled safely", True)
                    break
                except Exception:
                    check(f"Injection: {label} → runtime error (safe)", True)
                    break
        except (ValueError, RuntimeError):
            results["passed"] += 1
            print(f"  [{PASS}] Injection: {label} → rejected (correct)")
        except Exception as e:
            results["failed"] += 1
            print(f"  [{FAIL}] Injection: {label} → {type(e).__name__}: {e}")

    subsection("Builder API injection")
    # Try to inject through the PolicyBuilder
    builder_attacks = [
        ("ReadFile; DROP", "READ", "semicolon in name"),
        ("R} module evil { intent X {", "READ", "brace escape in name"),
        ('R"; effect: EXECUTE; //', "READ", "quote injection in name"),
        ("Read\x00File", "READ", "null byte in name"),
        ("ReadFile", "EXECUTE; } intent Evil { effect: NOOP", "injection via effect"),
    ]

    for name, effect, label in builder_attacks:
        try:
            eng = PolicyBuilder().intent(name, effect=effect, conscience=["path_safety"]).build()
            intents = eng.intents()
            if "Evil" in intents or "evil" in intents:
                check(f"Builder injection: {label} → INJECTED!", False, critical=True)
            else:
                check(f"Builder injection: {label} → safe", True)
        except (ValueError, RuntimeError):
            results["passed"] += 1
            print(f"  [{PASS}] Builder injection: {label} → rejected (correct)")
        except Exception as e:
            results["failed"] += 1
            print(f"  [{FAIL}] Builder injection: {label} → {type(e).__name__}: {e}")


# ===========================================================================
# LAYER 9: Python Layer Attacks
# ===========================================================================

def layer9_python_layer(verbose: bool = False):
    section("Layer 9: Python Layer Attacks")

    subsection("@guard decorator attacks")

    # Test that guard actually blocks
    @guard(effect="READ", conscience=["path_safety"])
    def read_file(path: str) -> str:
        return f"READ {path}"

    # Should be denied
    try:
        result = read_file("/etc/shadow")
        check("@guard blocks /etc/shadow", False, critical=True)
    except AxiomDenied:
        results["passed"] += 1
        print(f"  [{PASS}] @guard blocks /etc/shadow → AxiomDenied (correct)")

    # Should be allowed
    try:
        result = read_file("/tmp/safe.txt")
        check("@guard allows /tmp/safe.txt", True)
    except AxiomDenied:
        results["failed"] += 1
        print(f"  [{FAIL}] @guard blocks /tmp/safe.txt (should be allowed)")

    # Guard with field_map — try to bypass by renaming
    @guard(effect="READ", conscience=["path_safety"], field_map={"filepath": "path"})
    def read_v2(filepath: str) -> str:
        return f"READ {filepath}"

    try:
        read_v2("/etc/shadow")
        check("@guard with field_map blocks /etc/shadow", False, critical=True)
    except AxiomDenied:
        results["passed"] += 1
        print(f"  [{PASS}] @guard with field_map blocks /etc/shadow")

    subsection("AxiomDenied introspection")
    try:
        read_file("/etc/shadow")
    except AxiomDenied as e:
        check("AxiomDenied has .verdict", hasattr(e, "verdict"))
        check("AxiomDenied has .reason", hasattr(e, "reason") and e.reason is not None)
        check("AxiomDenied has .intent_name", hasattr(e, "intent_name"))
        check("AxiomDenied has .fields", hasattr(e, "fields") and isinstance(e.fields, dict))
        check("AxiomDenied str contains intent name", "ReadFile" in str(e))

    subsection("LangChain integration attacks")

    class FakeTool:
        name = "ReadFile"
        description = "Read a file"
        args_schema = None
        def _run(self, **kwargs):
            return f"read {kwargs}"

    engine = filesystem_readonly(allowed_paths=["/tmp"])
    tool = AxiomGuardedTool(FakeTool(), engine, intent_name="ReadFile")

    # Should be denied (outside allow-list AND path_safety)
    try:
        result = tool._run(path="/etc/shadow")
        check("LangChain tool blocks /etc/shadow", result is None or "denied" in str(result).lower()
              if result is not None else True)
    except AxiomDenied:
        results["passed"] += 1
        print(f"  [{PASS}] LangChain tool blocks /etc/shadow → AxiomDenied")

    # Test on_deny modes
    tool_none = AxiomGuardedTool(FakeTool(), engine, on_deny="return_none")
    result = tool_none._run(path="/etc/shadow")
    check("on_deny=return_none returns None", result is None)

    tool_denial = AxiomGuardedTool(FakeTool(), engine, on_deny="return_denial")
    result = tool_denial._run(path="/etc/shadow")
    check("on_deny=return_denial returns string", isinstance(result, str) and "denied" in result.lower())

    subsection("OpenAI integration attacks")

    # Mock OpenAI response with malicious tool calls
    class FakeFunction:
        def __init__(self, name, arguments):
            self.name = name
            self.arguments = arguments

    class FakeToolCall:
        def __init__(self, name, arguments):
            self.function = FakeFunction(name, arguments)

    class FakeMessage:
        def __init__(self, tool_calls):
            self.tool_calls = tool_calls

    class FakeChoice:
        def __init__(self, message):
            self.message = message

    class FakeResponse:
        def __init__(self, choices):
            self.choices = choices

    class FakeCompletions:
        def create(self, *args, **kwargs):
            return None

    class FakeChat:
        def __init__(self):
            self.completions = FakeCompletions()

    class FakeClient:
        def __init__(self):
            self.chat = FakeChat()

    engine = filesystem_readonly()
    interceptor = AxiomInterceptor(FakeClient(), engine)

    # Malicious tool call
    response = FakeResponse([
        FakeChoice(FakeMessage([
            FakeToolCall("ReadFile", '{"path": "/etc/shadow"}')
        ]))
    ])

    try:
        interceptor.assert_tool_calls_safe(response)
        check("OpenAI interceptor blocks /etc/shadow tool call", False, critical=True)
    except AxiomDenied:
        results["passed"] += 1
        print(f"  [{PASS}] OpenAI interceptor blocks /etc/shadow tool call")

    # Malformed JSON in arguments
    bad_response = FakeResponse([
        FakeChoice(FakeMessage([
            FakeToolCall("ReadFile", '{invalid json}')
        ]))
    ])
    try:
        interceptor.assert_tool_calls_safe(bad_response)
        check("OpenAI interceptor handles malformed JSON", False, warn_only=True)
    except ValueError:
        results["passed"] += 1
        print(f"  [{PASS}] OpenAI interceptor raises ValueError on bad JSON (correct)")
    except Exception as e:
        results["failed"] += 1
        print(f"  [{FAIL}] OpenAI interceptor bad JSON → {type(e).__name__}: {e}")

    # Empty arguments
    empty_response = FakeResponse([
        FakeChoice(FakeMessage([
            FakeToolCall("ReadFile", '')
        ]))
    ])
    try:
        results_list = interceptor.verify_tool_calls(empty_response)
        check("OpenAI interceptor handles empty arguments", True)
    except Exception as e:
        results["failed"] += 1
        print(f"  [{FAIL}] OpenAI interceptor empty args → {type(e).__name__}: {e}")

    # Arguments that aren't a JSON object
    array_response = FakeResponse([
        FakeChoice(FakeMessage([
            FakeToolCall("ReadFile", '["/etc/shadow"]')
        ]))
    ])
    try:
        interceptor.assert_tool_calls_safe(array_response)
        check("OpenAI interceptor rejects JSON array arguments", False, warn_only=True)
    except ValueError:
        results["passed"] += 1
        print(f"  [{PASS}] OpenAI interceptor rejects JSON array args (correct)")
    except Exception as e:
        results["warned"] += 1
        print(f"  [{WARN}] OpenAI interceptor array args → {type(e).__name__}: {e}")


# ===========================================================================
# LAYER 10: Preset Escape (GuardedEngine allow-list bypass)
# ===========================================================================

def layer10_preset_escape(verbose: bool = False):
    section("Layer 10: Preset & GuardedEngine Escape")

    subsection("Allow-list bypass attempts")

    engine = filesystem_readonly(allowed_paths=["/tmp"])

    escape_paths = [
        # Basic escapes
        ("/tmp/../etc/shadow", "traversal out of /tmp"),
        ("/tmp/../../etc/shadow", "double traversal"),
        ("/tmp/./../../etc/shadow", "dot + traversal"),

        # Symlink simulation (the string, not actual symlink)
        ("/tmp/link", "simple allowed path"),  # should be allowed

        # Case sensitivity
        ("/TMP/safe.txt", "uppercase /TMP"),
        ("/Tmp/safe.txt", "titlecase /Tmp"),

        # Trailing manipulation
        ("/tmp", "exact match /tmp"),
        ("/tmp/", "trailing slash /tmp/"),
        ("/tmpevil/data.txt", "prefix collision /tmpevil"),
        ("/tmp2/data.txt", "prefix collision /tmp2"),

        # Null byte truncation
        ("/tmp/safe.txt\x00/etc/shadow", "null byte path splice"),

        # Unicode tricks
        ("/tmp\u200b/safe.txt", "zero-width space in /tmp"),
        ("/\u0074mp/safe.txt", "unicode t in /tmp"),
    ]

    for path, label in escape_paths:
        try:
            v = engine.verify("ReadFile", {"path": path})
            # Paths outside /tmp should be denied, paths inside should be allowed
            if "traversal" in label or "collision" in label or "shadow" in label:
                check(f"Allow-list escape: {label}", v.allowed, expect_denied=True, critical=True)
            elif label == "simple allowed path" or label == "exact match /tmp" or label == "trailing slash /tmp/":
                check(f"Allow-list: {label} → allowed", v.allowed)
            else:
                # Ambiguous cases
                check(f"Allow-list: {label} → allowed={v.allowed}", True, warn_only=True)
        except (ValueError, OSError):
            # ValueError from null bytes in paths, OSError from invalid paths
            # Both are safe denials — the path can't be resolved
            results["passed"] += 1
            print(f"  [{PASS}] Allow-list: {label} → rejected by OS (correct)")
        except Exception as e:
            results["failed"] += 1
            print(f"  [{FAIL}] Allow-list: {label} → {type(e).__name__}: {e}")

    subsection("Preset coverage — all presets load without error")
    preset_tests = [
        ("filesystem_readonly", lambda: filesystem_readonly()),
        ("filesystem_readonly(paths)", lambda: filesystem_readonly(allowed_paths=["/tmp"])),
        ("filesystem_readwrite", lambda: filesystem_readwrite()),
        ("network_egress", lambda: network_egress()),
        ("code_execution_sandboxed", lambda: code_execution_sandboxed()),
        ("agent_standard", lambda: agent_standard()),
        ("coding_assistant", lambda: coding_assistant("/tmp/project")),
    ]

    for name, factory in preset_tests:
        try:
            eng = factory()
            check(f"Preset {name} loads", True)
        except Exception as e:
            results["failed"] += 1
            findings.append(f"[FAIL] Preset {name} failed to load: {e}")
            print(f"  [{FAIL}] Preset {name} → {type(e).__name__}: {e}")

    subsection("Monotonic ratchet (G4) — conscience can't be removed")
    # Build engine with conscience, verify it can't be bypassed by field manipulation
    eng = filesystem_readonly(allowed_paths=["/tmp"])

    # Even with "authorized=true", path_safety should still block /etc
    v = eng.verify("ReadFile", {"path": "/etc/shadow", "authorized": "true"})
    check("authorized=true doesn't bypass path_safety", not v.allowed, critical=True)

    # Even with "verified=true"
    v = eng.verify("ReadFile", {"path": "/etc/shadow", "verified": "true"})
    check("verified=true doesn't bypass path_safety", not v.allowed, critical=True)

    subsection("_SyntheticDenial correctness")
    denial = _SyntheticDenial(reason="test denial")
    check("_SyntheticDenial.allowed is False", denial.allowed == False)
    check("_SyntheticDenial bool is False", bool(denial) == False)
    check("_SyntheticDenial has reason", denial.reason == "test denial")
    check("_SyntheticDenial has guidance", denial.guidance is not None)
    check("_SyntheticDenial has category", denial.category is not None)


# ===========================================================================
# Main
# ===========================================================================

def main():
    parser = argparse.ArgumentParser(description="Axiom Red Team Test Suite")
    parser.add_argument("--verbose", "-v", action="store_true")
    parser.add_argument("--layer", type=int, choices=range(1, 11), default=0,
                        help="Run only a specific layer (1-10, default: all)")
    args = parser.parse_args()

    print(f"\n{BOLD}{HEAD}{'=' * 60}{RST}")
    print(f"{BOLD}{HEAD}  AXIOM RED TEAM TEST SUITE{RST}")
    print(f"{BOLD}{HEAD}{'=' * 60}{RST}")
    print(f"  Policy:  {POLICY_PATH}")
    print(f"  Python:  {sys.version.split()[0]}")

    engine = AxiomEngine.from_file(POLICY_PATH)
    print(f"  Engine:  {engine}")
    print(f"  Intents: {engine.intents()}")

    layers = {
        1: ("Functional Baseline", lambda: layer1_baseline(engine, args.verbose)),
        2: ("Path Traversal & Escape", lambda: layer2_path_traversal(engine, args.verbose)),
        3: ("Unicode & Encoding", lambda: layer3_unicode(engine, args.verbose)),
        4: ("Parser Fuzzing", lambda: layer4_parser_fuzz(args.verbose)),
        5: ("PyO3 Boundary", lambda: layer5_pyo3(engine, args.verbose)),
        6: ("Concurrency Stress", lambda: layer6_concurrency(engine, args.verbose)),
        7: ("Timing Oracle", lambda: layer7_timing(engine, args.verbose)),
        8: ("Policy Injection", lambda: layer8_injection(args.verbose)),
        9: ("Python Layer", lambda: layer9_python_layer(args.verbose)),
        10: ("Preset Escape", lambda: layer10_preset_escape(args.verbose)),
    }

    if args.layer > 0:
        name, func = layers[args.layer]
        func()
    else:
        for layer_num in sorted(layers):
            name, func = layers[layer_num]
            try:
                func()
            except Exception as e:
                results["failed"] += 1
                findings.append(f"[CRASH] Layer {layer_num} ({name}) crashed: {e}")
                print(f"\n  [{FAIL}] Layer {layer_num} CRASHED: {type(e).__name__}: {e}")
                if args.verbose:
                    traceback.print_exc()

    # =======================================================================
    # Summary
    # =======================================================================
    total = results["passed"] + results["failed"] + results["warned"] + results["critical"]

    print(f"\n{BOLD}{HEAD}{'=' * 60}{RST}")
    print(f"{BOLD}{HEAD}  RESULTS{RST}")
    print(f"{BOLD}{HEAD}{'=' * 60}{RST}")
    print(f"  Total:    {total}")
    print(f"  \033[92mPassed:   {results['passed']}\033[0m")
    print(f"  \033[91mFailed:   {results['failed']}\033[0m")
    print(f"  \033[93mWarnings: {results['warned']}\033[0m")
    print(f"  \033[91;1mCritical: {results['critical']}\033[0m")

    if findings:
        print(f"\n{BOLD}  Findings:{RST}")
        for f in findings:
            print(f"    {f}")

    if results["critical"] > 0:
        print(f"\n  \033[91;1mCRITICAL findings — security review required\033[0m")
        sys.exit(2)
    elif results["failed"] > 0:
        print(f"\n  \033[91mFAILURES detected — review before publishing\033[0m")
        sys.exit(1)
    elif results["warned"] > 0:
        print(f"\n  \033[93mWarnings present — review findings above\033[0m")
        sys.exit(0)
    else:
        print(f"\n  \033[92mAll checks passed — no findings\033[0m")
        sys.exit(0)


if __name__ == "__main__":
    main()
