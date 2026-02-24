(* G3_Determinism.v — G3: Same input always produces same verdict

   G3: Axiom is fully deterministic. Given the same intent and the
   same fields, verify() always returns exactly the same verdict.
   No randomness, no timing dependencies, no external state.

   This property is what makes Axiom auditable: a verdict can be
   reproduced, logged, and verified after the fact by anyone with
   the same policy and inputs.

   We prove:
     - G3_determinism: trivial reflexivity (same call = same result)
     - G3_determinism_equal_inputs: equal arguments → equal verdicts
     - G3_thread_safety: concurrent calls with equal inputs agree
       (in a purely functional model, parallelism is irrelevant)
*)

Require Import AxiomTypes.
Require Import AxiomVerify.

(* G3a: verify is deterministic — a call equals itself. *)
Theorem G3_determinism : forall (intent : Intent) (fields : Fields),
  verify intent fields = verify intent fields.
Proof.
  intros intent fields.
  reflexivity.
Qed.

(* G3b: Equal inputs always produce equal outputs.
   No external state can cause two calls with identical arguments
   to diverge. *)
Theorem G3_determinism_equal_inputs :
  forall (i1 i2 : Intent) (f1 f2 : Fields),
  i1 = i2 ->
  f1 = f2 ->
  verify i1 f1 = verify i2 f2.
Proof.
  intros i1 i2 f1 f2 Hi Hf.
  subst.
  reflexivity.
Qed.

(* G3c: Thread safety follows from purity.
   In a functional model, two "concurrent" calls are just two calls.
   They produce the same result because there is no shared mutable state. *)
Theorem G3_thread_safety :
  forall (intent : Intent) (fields : Fields),
  let v1 := verify intent fields in
  let v2 := verify intent fields in
  v1 = v2.
Proof.
  intros intent fields v1 v2.
  unfold v1, v2.
  reflexivity.
Qed.
