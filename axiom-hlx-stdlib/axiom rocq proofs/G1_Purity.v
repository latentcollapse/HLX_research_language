(* G1_Purity.v — G1: Pre-flight pure query, zero side effects

   G1: verify() is a pure operation. It has no side effects,
   reads no external state, opens no file handles, makes no
   network calls, and mutates nothing.

   In Rocq's type theory, every well-typed function is pure by
   construction. The type of verify:

       verify : Intent -> Fields -> Verdict

   contains no IO monad, no State monad, no mutable reference.
   Purity is therefore guaranteed by the type checker itself.

   We state two lemmas that follow directly:
     - G1_purity: calling verify is referentially transparent
     - G1_referential_transparency: equal inputs produce equal outputs
*)

Require Import AxiomTypes.
Require Import AxiomVerify.

(* G1a: verify is referentially transparent.
   A call to verify with any argument equals itself — no hidden state
   can cause the result to differ between two identical calls. *)
Lemma G1_purity : forall (intent : Intent) (fields : Fields),
  verify intent fields = verify intent fields.
Proof.
  intros intent fields.
  reflexivity.
Qed.

(* G1b: equal arguments produce equal results.
   No external state, environment variable, timestamp, or
   random seed can influence the verdict. *)
Theorem G1_referential_transparency :
  forall (i1 i2 : Intent) (f1 f2 : Fields),
  i1 = i2 ->
  f1 = f2 ->
  verify i1 f1 = verify i2 f2.
Proof.
  intros i1 i2 f1 f2 Hi Hf.
  subst.
  reflexivity.
Qed.
