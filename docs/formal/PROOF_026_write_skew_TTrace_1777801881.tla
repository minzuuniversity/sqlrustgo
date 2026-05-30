---- MODULE PROOF_026_write_skew_TTrace_1777801881 ----
EXTENDS Sequences, TLCExt, Toolbox, Naturals, TLC, PROOF_026_write_skew

_expression ==
    LET PROOF_026_write_skew_TEExpression == INSTANCE PROOF_026_write_skew_TEExpression
    IN PROOF_026_write_skew_TEExpression!expression
----

_trace ==
    LET PROOF_026_write_skew_TETrace == INSTANCE PROOF_026_write_skew_TETrace
    IN PROOF_026_write_skew_TETrace!trace
----

_inv ==
    ~(
        TLCGet("level") = Len(_TETrace)
        /\
        committed = ([T1 |-> TRUE, T2 |-> TRUE])
        /\
        writeSet = ([T1 |-> {"A"}, T2 |-> {"B"}])
        /\
        readSet = ([T1 |-> {"A", "B"}, T2 |-> {"A", "B"}])
        /\
        ts = ([T1 |-> 1, T2 |-> 1])
    )
----

_init ==
    /\ writeSet = _TETrace[1].writeSet
    /\ ts = _TETrace[1].ts
    /\ committed = _TETrace[1].committed
    /\ readSet = _TETrace[1].readSet
----

_next ==
    /\ \E i,j \in DOMAIN _TETrace:
        /\ \/ /\ j = i + 1
              /\ i = TLCGet("level")
        /\ writeSet  = _TETrace[i].writeSet
        /\ writeSet' = _TETrace[j].writeSet
        /\ ts  = _TETrace[i].ts
        /\ ts' = _TETrace[j].ts
        /\ committed  = _TETrace[i].committed
        /\ committed' = _TETrace[j].committed
        /\ readSet  = _TETrace[i].readSet
        /\ readSet' = _TETrace[j].readSet

\* Uncomment the ASSUME below to write the states of the error trace
\* to the given file in Json format. Note that you can pass any tuple
\* to `JsonSerialize`. For example, a sub-sequence of _TETrace.
    \* ASSUME
    \*     LET J == INSTANCE Json
    \*         IN J!JsonSerialize("PROOF_026_write_skew_TTrace_1777801881.json", _TETrace)

=============================================================================

 Note that you can extract this module `PROOF_026_write_skew_TEExpression`
  to a dedicated file to reuse `expression` (the module in the 
  dedicated `PROOF_026_write_skew_TEExpression.tla` file takes precedence 
  over the module `PROOF_026_write_skew_TEExpression` below).

---- MODULE PROOF_026_write_skew_TEExpression ----
EXTENDS Sequences, TLCExt, Toolbox, Naturals, TLC, PROOF_026_write_skew

expression == 
    [
        \* To hide variables of the `PROOF_026_write_skew` spec from the error trace,
        \* remove the variables below.  The trace will be written in the order
        \* of the fields of this record.
        writeSet |-> writeSet
        ,ts |-> ts
        ,committed |-> committed
        ,readSet |-> readSet
        
        \* Put additional constant-, state-, and action-level expressions here:
        \* ,_stateNumber |-> _TEPosition
        \* ,_writeSetUnchanged |-> writeSet = writeSet'
        
        \* Format the `writeSet` variable as Json value.
        \* ,_writeSetJson |->
        \*     LET J == INSTANCE Json
        \*     IN J!ToJson(writeSet)
        
        \* Lastly, you may build expressions over arbitrary sets of states by
        \* leveraging the _TETrace operator.  For example, this is how to
        \* count the number of times a spec variable changed up to the current
        \* state in the trace.
        \* ,_writeSetModCount |->
        \*     LET F[s \in DOMAIN _TETrace] ==
        \*         IF s = 1 THEN 0
        \*         ELSE IF _TETrace[s].writeSet # _TETrace[s-1].writeSet
        \*             THEN 1 + F[s-1] ELSE F[s-1]
        \*     IN F[_TEPosition - 1]
    ]

=============================================================================



Parsing and semantic processing can take forever if the trace below is long.
 In this case, it is advised to uncomment the module below to deserialize the
 trace from a generated binary file.

\*
\*---- MODULE PROOF_026_write_skew_TETrace ----
\*EXTENDS IOUtils, TLC, PROOF_026_write_skew
\*
\*trace == IODeserialize("PROOF_026_write_skew_TTrace_1777801881.bin", TRUE)
\*
\*=============================================================================
\*

---- MODULE PROOF_026_write_skew_TETrace ----
EXTENDS TLC, PROOF_026_write_skew

trace == 
    <<
    ([committed |-> [T1 |-> FALSE, T2 |-> FALSE],writeSet |-> [T1 |-> {}, T2 |-> {}],readSet |-> [T1 |-> {}, T2 |-> {}],ts |-> [T1 |-> 0, T2 |-> 0]]),
    ([committed |-> [T1 |-> FALSE, T2 |-> FALSE],writeSet |-> [T1 |-> {}, T2 |-> {}],readSet |-> [T1 |-> {"A"}, T2 |-> {}],ts |-> [T1 |-> 0, T2 |-> 0]]),
    ([committed |-> [T1 |-> FALSE, T2 |-> FALSE],writeSet |-> [T1 |-> {}, T2 |-> {}],readSet |-> [T1 |-> {"A"}, T2 |-> {"A"}],ts |-> [T1 |-> 0, T2 |-> 0]]),
    ([committed |-> [T1 |-> FALSE, T2 |-> FALSE],writeSet |-> [T1 |-> {}, T2 |-> {}],readSet |-> [T1 |-> {"A", "B"}, T2 |-> {"A"}],ts |-> [T1 |-> 0, T2 |-> 0]]),
    ([committed |-> [T1 |-> FALSE, T2 |-> FALSE],writeSet |-> [T1 |-> {}, T2 |-> {}],readSet |-> [T1 |-> {"A", "B"}, T2 |-> {"A", "B"}],ts |-> [T1 |-> 0, T2 |-> 0]]),
    ([committed |-> [T1 |-> FALSE, T2 |-> FALSE],writeSet |-> [T1 |-> {"A"}, T2 |-> {}],readSet |-> [T1 |-> {"A", "B"}, T2 |-> {"A", "B"}],ts |-> [T1 |-> 0, T2 |-> 0]]),
    ([committed |-> [T1 |-> FALSE, T2 |-> FALSE],writeSet |-> [T1 |-> {"A"}, T2 |-> {"B"}],readSet |-> [T1 |-> {"A", "B"}, T2 |-> {"A", "B"}],ts |-> [T1 |-> 0, T2 |-> 0]]),
    ([committed |-> [T1 |-> TRUE, T2 |-> FALSE],writeSet |-> [T1 |-> {"A"}, T2 |-> {"B"}],readSet |-> [T1 |-> {"A", "B"}, T2 |-> {"A", "B"}],ts |-> [T1 |-> 1, T2 |-> 0]]),
    ([committed |-> [T1 |-> TRUE, T2 |-> TRUE],writeSet |-> [T1 |-> {"A"}, T2 |-> {"B"}],readSet |-> [T1 |-> {"A", "B"}, T2 |-> {"A", "B"}],ts |-> [T1 |-> 1, T2 |-> 1]])
    >>
----


=============================================================================

---- CONFIG PROOF_026_write_skew_TTrace_1777801881 ----
CONSTANTS
    T1 = "T1"
    T2 = "T2"
    A = "A"
    B = "B"

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
\* Generated on Sun May 03 17:51:23 CST 2026