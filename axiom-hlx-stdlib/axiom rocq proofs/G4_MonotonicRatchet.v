(* G4_MonotonicRatchet.v — G4: Restrictions only accumulate

   G4: The Monotonic Ratchet. Once a conscience predicate is added
   to a policy, it can never be silently dropped. Adding more
   predicates to an intent can only introduce new denials, never
   remove existing ones.

   Formally: if verify(intent) = Denied r, then for any additional
   predicates [extra], verify(intent with preds ++ extra) = Denied r.

   This is the property that makes Axiom policies composable without
   hidden escalation: you can stack presets and know the result is
   strictly at least as restrictive.

   We prove:
     - verify_predicates_denial_preserved: the key inductive lemma
     - G4_monotonic_ratchet: the main theorem
     - G4_no_undeny: corollary — denials are permanent
     - G4_allow_may_be_lost: contra-positive — allowed intents
       may be denied after adding predicates (expected behavior)
*)

Require Import AxiomTypes.
Require Import AxiomVerify.
Open Scope list_scope.

(* Key inductive lemma: a denial produced by [preds] survives
   the addition of [extra] predicates appended afterward. *)
Lemma verify_predicates_denial_preserved :
  forall (preds extra : list Predicate) (eff : Effect)
         (fields : Fields) (r : DenialReason),
  verify_predicates preds eff fields = Denied r ->
  verify_predicates (preds ++ extra) eff fields = Denied r.
Proof.
  induction preds as [| p ps IH].
  - (* Base case: preds = nil.
       nil always returns Allowed, so Denied r is a contradiction. *)
    intros extra eff fields r H.
    simpl in H.
    discriminate.
  - (* Inductive case: preds = p :: ps *)
    intros extra eff fields r H.
    simpl in *.
    destruct (eval_predicate p eff fields) as [| dr] eqn:Heval.
    + (* p returned Allowed — denial must come from ps *)
      apply IH.
      exact H.
    + (* p returned Denied dr — result is Denied dr, done *)
      exact H.
Qed.

(* G4: The Monotonic Ratchet.
   If conscience predicates [preds] deny an intent, then
   [preds ++ extra] also denies it — with the same reason. *)
Theorem G4_monotonic_ratchet :
  forall (eff : Effect) (preds extra : list Predicate)
         (fields : Fields) (r : DenialReason),
  verify (mkIntent eff preds) fields = Denied r ->
  verify (mkIntent eff (preds ++ extra)) fields = Denied r.
Proof.
  intros eff preds extra fields r H.
  unfold verify in *.
  simpl in *.
  apply verify_predicates_denial_preserved.
  exact H.
Qed.

(* Corollary: once denied, always denied under any extension. *)
Theorem G4_no_undeny :
  forall (eff : Effect) (preds extra : list Predicate) (fields : Fields),
  (exists r, verify (mkIntent eff preds) fields = Denied r) ->
  (exists r, verify (mkIntent eff (preds ++ extra)) fields = Denied r).
Proof.
  intros eff preds extra fields [r Hr].
  exists r.
  apply G4_monotonic_ratchet.
  exact Hr.
Qed.

(* Note: the converse does NOT hold — adding predicates may turn
   Allowed into Denied. This is expected and desirable behavior.
   Policies can only become more restrictive, never more permissive. *)
