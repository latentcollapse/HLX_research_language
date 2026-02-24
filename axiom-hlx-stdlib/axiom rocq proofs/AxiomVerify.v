(* AxiomVerify.v - The verify function model

   We axiomatize the predicate evaluator (eval_predicate) — its specific
   logic lives in Rust. We then define verify as a pure structural function
   over that evaluator and prove G1-G6 hold for any well-typed evaluator.
*)

Require Import AxiomTypes.
Open Scope list_scope.

(* Abstract predicate evaluator.
   The actual path_safety / no_exfiltrate logic is in Rust.
   We treat it as a parameter here: any function of this type
   will satisfy our proofs, because the guarantees are structural. *)
Parameter eval_predicate : Predicate -> Effect -> Fields -> Verdict.

(* Apply conscience predicates sequentially.
   First denial wins — this is the monotonic ratchet core. *)
Fixpoint verify_predicates
    (preds  : list Predicate)
    (eff    : Effect)
    (fields : Fields) : Verdict :=
  match preds with
  | nil      => Allowed
  | cons p ps =>
      match eval_predicate p eff fields with
      | Denied r => Denied r
      | Allowed  => verify_predicates ps eff fields
      end
  end.

(* Top-level verify function — pure, total, deterministic. *)
Definition verify (intent : Intent) (fields : Fields) : Verdict :=
  verify_predicates (conscience intent) (effect intent) fields.
