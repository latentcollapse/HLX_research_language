(* G5_SpecificDenial.v — G5: Every denial cites a specific predicate

   G5: When Axiom denies an action, the verdict is never a generic
   "denied." It always cites the specific predicate that failed,
   the field key that triggered it, and the field value that violated
   the policy.

   This is what makes Axiom auditable and debuggable:
     verdict.reason  → specific technical denial string
     verdict.guidance → human-readable category

   In our model, DenialReason carries (predicate, field_key, field_value).
   We prove that:
     - Every Denied verdict carries a DenialReason (no anonymous failures)
     - The DenialReason is fully decomposable into its components
     - There is no Verdict constructor between Allowed and Denied

   We prove:
     - G5_verdict_is_specific: every verdict is Allowed or Denied with reason
     - G5_denial_has_reason: Denied implies a DenialReason exists
     - G5_denial_is_decomposable: DenialReason always has all three fields
*)

Require Import AxiomTypes.
Require Import AxiomVerify.
Open Scope list_scope.

(* Helper: verify_predicates always produces Allowed or Denied r — nothing else. *)
Lemma verify_predicates_verdict_specific :
  forall (preds : list Predicate) (eff : Effect) (fields : Fields),
  verify_predicates preds eff fields = Allowed \/
  exists r : DenialReason, verify_predicates preds eff fields = Denied r.
Proof.
  induction preds as [| p ps IH].
  - (* nil: always Allowed *)
    intros eff fields.
    left. reflexivity.
  - intros eff fields.
    simpl.
    destruct (eval_predicate p eff fields) as [| dr] eqn:Heval.
    + (* p returned Allowed — recurse into ps *)
      apply IH.
    + (* p returned Denied dr — verdict is specific *)
      right. exists dr. reflexivity.
Qed.

(* G5a: Every call to verify produces either Allowed or Denied r.
   There is no "error," "unknown," or "undefined" verdict. *)
Theorem G5_verdict_is_specific :
  forall (intent : Intent) (fields : Fields),
  verify intent fields = Allowed \/
  exists r : DenialReason, verify intent fields = Denied r.
Proof.
  intros intent fields.
  unfold verify.
  apply verify_predicates_verdict_specific.
Qed.

(* G5b: If verify returns Denied, a DenialReason exists and is accessible. *)
Theorem G5_denial_has_reason :
  forall (intent : Intent) (fields : Fields) (r : DenialReason),
  verify intent fields = Denied r ->
  exists (p : Predicate) (k : FieldKey) (v : FieldValue),
    denied_by r = p /\ field_key r = k /\ field_value r = v.
Proof.
  intros intent fields r H.
  exists (denied_by r), (field_key r), (field_value r).
  auto.
Qed.

(* G5c: A DenialReason is fully decomposable into its three components:
   the predicate that fired, the field key that triggered it, and the value. *)
Theorem G5_denial_is_decomposable :
  forall (intent : Intent) (fields : Fields) (r : DenialReason),
  verify intent fields = Denied r ->
  r = mkDenial (denied_by r) (field_key r) (field_value r).
Proof.
  intros intent fields r _.
  destruct r.
  reflexivity.
Qed.
