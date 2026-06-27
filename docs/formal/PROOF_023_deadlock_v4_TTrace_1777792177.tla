---- MODULE PROOF_023_deadlock_v4_TTrace_1777792177 ----
EXTENDS Sequences, TLCExt, Toolbox, Naturals, TLC, PROOF_023_deadlock_v4, PROOF_023_deadlock_v4_TEConstants

_expression ==
    LET PROOF_023_deadlock_v4_TEExpression == INSTANCE PROOF_023_deadlock_v4_TEExpression
    IN PROOF_023_deadlock_v4_TEExpression!expression
----

_trace ==
    LET PROOF_023_deadlock_v4_TETrace == INSTANCE PROOF_023_deadlock_v4_TETrace
    IN PROOF_023_deadlock_v4_TETrace!trace
----

_inv ==
    ~(
        TLCGet("level") = Len(_TETrace)
        /\
        txnHoldLocks = ((T1 :> {} @@ T2 :> {} @@ T3 :> {}))
        /\
        txnWaitFor = ((T1 :> {} @@ T2 :> {T1} @@ T3 :> {}))
    )
----

_init ==
    /\ txnHoldLocks = _TETrace[1].txnHoldLocks
    /\ txnWaitFor = _TETrace[1].txnWaitFor
----

_next ==
    /\ \E i,j \in DOMAIN _TETrace:
        /\ \/ /\ j = i + 1
              /\ i = TLCGet("level")
        /\ txnHoldLocks  = _TETrace[i].txnHoldLocks
        /\ txnHoldLocks' = _TETrace[j].txnHoldLocks
        /\ txnWaitFor  = _TETrace[i].txnWaitFor
        /\ txnWaitFor' = _TETrace[j].txnWaitFor

\* Uncomment the ASSUME below to write the states of the error trace
\* to the given file in Json format. Note that you can pass any tuple
\* to `JsonSerialize`. For example, a sub-sequence of _TETrace.
    \* ASSUME
    \*     LET J == INSTANCE Json
    \*         IN J!JsonSerialize("PROOF_023_deadlock_v4_TTrace_1777792177.json", _TETrace)

=============================================================================

 Note that you can extract this module `PROOF_023_deadlock_v4_TEExpression`
  to a dedicated file to reuse `expression` (the module in the 
  dedicated `PROOF_023_deadlock_v4_TEExpression.tla` file takes precedence 
  over the module `PROOF_023_deadlock_v4_TEExpression` below).

---- MODULE PROOF_023_deadlock_v4_TEExpression ----
EXTENDS Sequences, TLCExt, Toolbox, Naturals, TLC, PROOF_023_deadlock_v4, PROOF_023_deadlock_v4_TEConstants

expression == 
    [
        \* To hide variables of the `PROOF_023_deadlock_v4` spec from the error trace,
        \* remove the variables below.  The trace will be written in the order
        \* of the fields of this record.
        txnHoldLocks |-> txnHoldLocks
        ,txnWaitFor |-> txnWaitFor
        
        \* Put additional constant-, state-, and action-level expressions here:
        \* ,_stateNumber |-> _TEPosition
        \* ,_txnHoldLocksUnchanged |-> txnHoldLocks = txnHoldLocks'
        
        \* Format the `txnHoldLocks` variable as Json value.
        \* ,_txnHoldLocksJson |->
        \*     LET J == INSTANCE Json
        \*     IN J!ToJson(txnHoldLocks)
        
        \* Lastly, you may build expressions over arbitrary sets of states by
        \* leveraging the _TETrace operator.  For example, this is how to
        \* count the number of times a spec variable changed up to the current
        \* state in the trace.
        \* ,_txnHoldLocksModCount |->
        \*     LET F[s \in DOMAIN _TETrace] ==
        \*         IF s = 1 THEN 0
        \*         ELSE IF _TETrace[s].txnHoldLocks # _TETrace[s-1].txnHoldLocks
        \*             THEN 1 + F[s-1] ELSE F[s-1]
        \*     IN F[_TEPosition - 1]
    ]

=============================================================================



Parsing and semantic processing can take forever if the trace below is long.
 In this case, it is advised to uncomment the module below to deserialize the
 trace from a generated binary file.

\*
\*---- MODULE PROOF_023_deadlock_v4_TETrace ----
\*EXTENDS IOUtils, TLC, PROOF_023_deadlock_v4, PROOF_023_deadlock_v4_TEConstants
\*
\*trace == IODeserialize("PROOF_023_deadlock_v4_TTrace_1777792177.bin", TRUE)
\*
\*=============================================================================
\*

---- MODULE PROOF_023_deadlock_v4_TETrace ----
EXTENDS TLC, PROOF_023_deadlock_v4, PROOF_023_deadlock_v4_TEConstants

trace == 
    <<
    ([txnHoldLocks |-> (T1 :> {} @@ T2 :> {} @@ T3 :> {}),txnWaitFor |-> (T1 :> {} @@ T2 :> {} @@ T3 :> {})]),
    ([txnHoldLocks |-> (T1 :> {K1} @@ T2 :> {} @@ T3 :> {}),txnWaitFor |-> (T1 :> {} @@ T2 :> {} @@ T3 :> {})]),
    ([txnHoldLocks |-> (T1 :> {K1} @@ T2 :> {} @@ T3 :> {}),txnWaitFor |-> (T1 :> {} @@ T2 :> {T1} @@ T3 :> {})]),
    ([txnHoldLocks |-> (T1 :> {} @@ T2 :> {} @@ T3 :> {}),txnWaitFor |-> (T1 :> {} @@ T2 :> {T1} @@ T3 :> {})])
    >>
----


=============================================================================

---- MODULE PROOF_023_deadlock_v4_TEConstants ----
EXTENDS PROOF_023_deadlock_v4

CONSTANTS K3

=============================================================================

---- CONFIG PROOF_023_deadlock_v4_TTrace_1777792177 ----
CONSTANTS
    T1 = T1
    T2 = T2
    T3 = T3
    K1 = K1
    K2 = K2
    K3 = K3
    K2 = K2
    T3 = T3
    K3 = K3
    K1 = K1
    T2 = T2
    T1 = T1

INVARIANT
    _inv

CHECK_DEADLOCK
    \* CHECK_DEADLOCK off because of PROPERTY or INVARIANT above.
    FALSE

INIT
    _init

NEXT
    _next

CONSTANT
    _TETrace <- _trace

ALIAS
    _expression
=============================================================================
\* Generated on Sun May 03 15:09:38 CST 2026