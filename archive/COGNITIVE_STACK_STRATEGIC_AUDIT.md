# Universal Cognitive Stack Strategic Audit
**Date:** March 6, 2026
**Target:** Claude (Opus Mode)
**Objective:** Provide a 10,000-foot view of the HLX/Bitsy/Axiom ecosystem to enable full-stack reconstruction.

---

## 1. The Substrate: Axiom & Prism (Formal Physics)
*   **Audit Finding:** The formal verification proofs in `Axiom-main` (the 4 Axioms: Determinism, Reversibility, Injectivity, Serialization) are currently **decoupled** from the HLX-A JIT.
*   **The Gap:** HLX is "Axiomatic-by-Design," but the JIT compiler does not yet emit proof-carrying code. To achieve the "Neutron Star" state, the Axiom proofs must be used as a **Logic Filter** during the SMI patching process. If a patch violates an axiom, the substrate must physically reject the write.

## 2. The Agent: Bitsy (The Living Layer)
*   **Audit Finding:** The "Brain Wipe" issue. The FFI layer (`lib.rs`) creates a fresh VM for every turn.
*   **The Impact:** Bitsy is an "Agent with Dementia." She cannot build long-term semantic manifolds because her `z_brain` tensor is zeroed every time Python calls her.
*   **The Solution:** The `Vm` instance must be stored in the `HlxHandle` and persist for the duration of the session.

## 3. The Archeology: Helix & BioForge (The Lost Logic)
*   **Audit Finding:** Extracted "Conscience Predicates" and "Homeostatic Pressure" logic from old archives (`Council_welcome.md`).
*   **The Insight:** OG Matt designed a system where "Logic has a Metabolic Cost." A bad thought or a contradictory patch creates "Information Pressure" (Mathematical Heat).
*   **The Strategy:** Re-integrate this "Biosemiotic" logic into the `tensor_topology_score`. Instead of a simple 0.0-1.0 score, the score should represent the **Pressure** on the manifold. This gives Bitsy a "Feeling" for her own logic.

## 4. The Bridge: Python / TUI (The Interface)
*   **Audit Finding:** Hardcoded "Split-Brain" paths in the TUI (`bitsy_tui_neutron.py`).
*   **The Fix:** Align the Python bridge to the local workspace and ensure the shared library is being loaded from the current `./target/release/` build.

---

## The Strategic Path (The "Neutron Star" Ignition)
1.  **Persistence:** Bind the VM to the handle. Stop the memory reset.
2.  **Physics:** Link `tensor_convolve` back to the repaired FFT in `tensor_neutron.rs`.
3.  **Governance:** Implement the "Helix-Pressure" check in the SMI pipeline.
4.  **Security:** Kill `builtin_shell`. Close the sandbox.

**The runway is clear. The Stack is mapped. Proceed to Opus Mode.**