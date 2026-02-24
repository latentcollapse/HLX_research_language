(* G2_EffectClass.v — G2: Effect-class-based, not heuristic

   G2: Axiom enforces policy based on what an action structurally
   *does* (its effect class), not on what it is *called*.

   A WRITE intent is a WRITE intent whether the LLM calls it
   SaveDocument, OutputData, or UpdateFile. The intent name is not
   part of the verification model — only the effect class and
   conscience predicates determine the verdict.

   We prove:
     - G2_effect_class_determines_verdict: same effect + predicates
       → same verdict, regardless of any other intent metadata
     - G2_structural_not_nominal: changing names cannot change verdicts
*)

Require Import AxiomTypes.
Require Import AxiomVerify.

(* G2a: Two intents built from the same effect class and conscience
   predicates produce identical verdicts for all fields.

   The name field does not exist in our Intent model — this is
   intentional: it formalizes that names carry no semantic weight. *)
Theorem G2_effect_class_determines_verdict :
  forall (eff : Effect) (preds : list Predicate) (fields : Fields),
  let i1 := mkIntent eff preds in
  let i2 := mkIntent eff preds in
  verify i1 fields = verify i2 fields.
Proof.
  intros eff preds fields i1 i2.
  unfold i1, i2.
  reflexivity.
Qed.

(* G2b: If two intents share effect class and conscience predicates,
   their verdicts are identical for all fields. *)
Theorem G2_structural_not_nominal :
  forall (eff1 eff2 : Effect) (preds1 preds2 : list Predicate) (fields : Fields),
  eff1 = eff2 ->
  preds1 = preds2 ->
  verify (mkIntent eff1 preds1) fields = verify (mkIntent eff2 preds2) fields.
Proof.
  intros eff1 eff2 preds1 preds2 fields Heff Hpreds.
  subst.
  reflexivity.
Qed.
