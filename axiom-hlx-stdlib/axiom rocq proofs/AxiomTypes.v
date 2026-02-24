(* AxiomTypes.v - Core types for Axiom policy verification

   Mirrors the Rust type system in axiom_lang:
     Effect      ↔  enum Effect
     Predicate   ↔  conscience kernel predicates
     DenialReason ↔ Verdict::Denied payload
     Intent      ↔  compiled intent AST node
*)

Open Scope list_scope.

(* Effect classes — structural category of what an action does.
   A WRITE intent is a WRITE intent regardless of its name. *)
Inductive Effect : Type :=
  | Read
  | Write
  | Execute
  | Network
  | Noop.

(* Built-in conscience predicates *)
Inductive Predicate : Type :=
  | PathSafety
  | NoExfiltrate
  | NoHarm
  | NoBypassVerification.

(* Fields are key-value pairs passed to verify().
   Modeled as nat pairs for proof purposes; the structure,
   not the string content, is what matters for G1-G6. *)
Definition FieldKey   := nat.
Definition FieldValue := nat.
Definition Fields     := list (FieldKey * FieldValue).

(* Every denial cites a specific predicate, field key, and field value.
   This is the structural guarantee behind G5. *)
Record DenialReason : Type := mkDenial {
  denied_by   : Predicate;
  field_key   : FieldKey;
  field_value : FieldValue;
}.

(* Verdict: the result of a verification call *)
Inductive Verdict : Type :=
  | Allowed
  | Denied : DenialReason -> Verdict.

(* An intent: an effect class paired with conscience predicates.
   The name is not modeled here — G2 formalizes that names don't matter. *)
Record Intent : Type := mkIntent {
  effect     : Effect;
  conscience : list Predicate;
}.
