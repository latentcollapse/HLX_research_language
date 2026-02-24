(* G6_Totality.v — G6: verify always produces a definite verdict

   G6: verify() is a total function. For every possible intent and
   every possible set of fields, it always returns either Allowed
   or Denied — never crashes, hangs, throws an exception, or
   returns an indeterminate result.

   This is the foundational guarantee that makes Axiom safe to call
   on any input without defensive wrapping. Combined with G1 (purity),
   a call to verify can never have unintended consequences and will
   always complete with a usable result.

   This guarantee holds because:
     1. verify_predicates is a structurally recursive Fixpoint
        (Rocq's termination checker verifies it terminates)
     2. eval_predicate is a total Parameter (assumed by axiom)
     3. The match arms cover all cases of Verdict

   We prove:
     - verify_predicates_total: the inner function always terminates
     - G6_totality: verify always returns some verdict
     - G6_verdict_exhaustive: the result is always Allowed or Denied r
*)

Require Import AxiomTypes.
Require Import AxiomVerify.
Open Scope list_scope.

(* Helper: verify_predicates terminates and returns a verdict
   for any list of predicates. *)
Lemma verify_predicates_total :
  forall (preds : list Predicate) (eff : Effect) (fields : Fields),
  exists v : Verdict, verify_predicates preds eff fields = v.
Proof.
  induction preds as [| p ps IH].
  - (* nil: always Allowed *)
    intros eff fields.
    exists Allowed. reflexivity.
  - intros eff fields.
    simpl.
    destruct (eval_predicate p eff fields) as [| dr] eqn:Heval.
    + (* Allowed: recurse *)
      apply IH.
    + (* Denied: done *)
      exists (Denied dr). reflexivity.
Qed.

(* G6: verify is total — always returns a verdict. *)
Theorem G6_totality :
  forall (intent : Intent) (fields : Fields),
  exists v : Verdict, verify intent fields = v.
Proof.
  intros intent fields.
  unfold verify.
  apply verify_predicates_total.
Qed.

(* G6 exhaustive form: the verdict is always Allowed or Denied r.
   There is no third case. Combined with G5, every denial is specific. *)
Theorem G6_verdict_exhaustive :
  forall (intent : Intent) (fields : Fields),
  verify intent fields = Allowed \/
  exists r : DenialReason, verify intent fields = Denied r.
Proof.
  intros intent fields.
  destruct (verify intent fields) as [| r] eqn:H.
  - left. reflexivity.
  - right. exists r. reflexivity.
Qed.
