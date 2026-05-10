# DSL (OPL) v1.0 - Diseno Tecnico v0.12

## Contexto

Hoy los Agent Packages se construyen en codigo Rust. Queremos que
un humano, o un meta-agente, escriba en texto plano:

```opl
CREATE AGENT invoice_processor
FOR "Procesar facturas de AFIP"
WITH
  domain = "finanzas",
  autonomy = "semi"

STEP login
  USES afip_login
  RISK HIGH
  HITL true

STEP fetch
  USES http_get
  INPUT { path = "/api/invoices", query = { status = "pending" } }

STEP validate
  USES form_validate
  VALIDATE [schema, business_rules]

ON timeout
  FALLBACK TO login
  RETRY 3
  BACKOFF EXPONENTIAL
```

Y el sistema genere el `AgentPackage` JSON equivalente.

## Arquitectura

```text
+-----------------------------------------+
|           OPL SOURCE                    |
|  - Texto plano, sintaxis declarativa    |
+-----------------------------------------+
|           LEXER + PARSER                |
|  - Tokeniza, genera AST                 |
|  - Valida sintaxis                      |
+-----------------------------------------+
|           SEMANTIC ANALYZER             |
|  - Valida referencias                   |
|  - Chequea tipos                        |
|  - Detecta ciclos                       |
+-----------------------------------------+
|           TRANSPILADOR                  |
|  - AST -> AgentPackage JSON             |
|  - Aplica defaults                      |
|  - Genera risk model                    |
+-----------------------------------------+
```

## Gramatica EBNF v1.0

```ebnf
program         ::= declaration+ EOF

declaration     ::= create_agent
                  | define_capability
                  | define_policy
                  | import_module

create_agent    ::= "CREATE" "AGENT" identifier
                    "FOR" string_literal
                    [ "EXTENDS" identifier ]
                    with_clause?
                    block?

with_clause     ::= "WITH" property_list

property_list   ::= property ( "," property )*

property        ::= identifier "=" value

value           ::= string_literal
                  | number_literal
                  | boolean_literal
                  | list_literal
                  | identifier

list_literal    ::= "[" [ value ( "," value )* ] "]"

block           ::= "{" statement+ "}"

statement       ::= step_declaration
                  | fallback_declaration
                  | constraint_declaration
                  | config_declaration

step_declaration ::= "STEP" identifier
                     "USES" identifier
                     [ "WITH" property_list ]
                     [ "RISK" risk_level ]
                     [ "VALIDATE" list_literal ]
                     [ "HITL" boolean_literal ]
                     [ "INPUT" value ]

risk_level      ::= "LOW" | "MEDIUM" | "HIGH" | "CRITICAL"

fallback_declaration ::= "ON" identifier
                         "FALLBACK" "TO" identifier
                         [ "RETRY" number_literal ]
                         [ "BACKOFF" backoff_type ]

backoff_type    ::= "FIXED" | "LINEAR" | "EXPONENTIAL"

constraint_declaration ::= "CONSTRAINT" identifier
                           "REQUIRES" list_literal
                           [ "ENFORCE" boolean_literal ]

config_declaration ::= "CONFIG" identifier "=" value

define_capability ::= "DEFINE" "CAPABILITY" identifier
                      "DOES" string_literal
                      [ "REQUIRES" list_literal ]
                      [ "EXPOSES" list_literal ]
                      [ "RISK" risk_level ]
                      [ "COST" string_literal ]

define_policy   ::= "DEFINE" "POLICY" identifier
                    "WHEN" condition
                    "THEN" action_list

condition       ::= expression ( "AND" | "OR" )* expression

expression      ::= identifier operator value

operator        ::= "==" | "!=" | ">" | "<" | ">=" | "<=" | "IN" | "MATCHES"

action_list     ::= action ( "," action )*

action          ::= identifier "=" value

import_module   ::= "IMPORT" string_literal [ "AS" identifier ]
```

## Estructuras del parser

```rust
pub struct OplParser;

pub struct Ast {
    pub declarations: Vec<Declaration>,
}

pub enum Declaration {
    Agent(AgentDecl),
    Capability(CapabilityDecl),
    Policy(PolicyDecl),
    Import(ImportDecl),
}

pub struct AgentDecl {
    pub id: String,
    pub mission: String,
    pub extends: Option<String>,
    pub properties: HashMap<String, AstValue>,
    pub steps: Vec<StepDecl>,
    pub fallbacks: Vec<FallbackDecl>,
    pub constraints: Vec<ConstraintDecl>,
    pub configs: HashMap<String, AstValue>,
}

pub struct StepDecl {
    pub id: String,
    pub capability: String,
    pub properties: HashMap<String, AstValue>,
    pub risk: Option<String>,
    pub validate: Vec<String>,
    pub hitl: bool,
    pub input: Option<AstValue>,
}

pub enum AstValue {
    String(String),
    Number(f64),
    Bool(bool),
    List(Vec<AstValue>),
    Identifier(String),
}
```

## Transpilacion a AgentPackage

```rust
impl Transpiler {
    pub fn transpile(ast: Ast) -> Result<AgentPackage, TranspileError> {
        let agent = ast.declarations.iter()
            .find_map(|d| match d {
                Declaration::Agent(a) => Some(a),
                _ => None,
            })
            .ok_or(TranspileError::NoAgentDeclaration)?;

        Ok(AgentPackage {
            id: agent.id.clone(),
            steps: agent.steps.iter().map(Self::transpile_step).collect(),
            max_steps: 100,
            risk_config: Self::transpile_risk(&agent.properties),
            global_config: Self::transpile_global(&agent.properties),
        })
    }
}
```

## Invariantes del DSL

1. Cada OPL valido transpila a exactamente un AgentPackage.
2. Cada AgentPackage puede serializarse desde OPL, con bidireccionalidad futura.
3. Error de parsing nunca genera AgentPackage parcial.
4. Defaults explicitos: si no se declara, se usa default documentado.
5. No logica de ejecucion en OPL: solo declaracion, no programacion.

## Tests planificados

| # | Test | Descripcion |
|---|------|-------------|
| 1 | `parse_minimal_agent` | CREATE AGENT + 1 STEP |
| 2 | `parse_full_agent` | Todas las construcciones |
| 3 | `parse_fallback` | ON ... FALLBACK TO ... |
| 4 | `transpile_to_package` | AST -> AgentPackage valido |
| 5 | `roundtrip_json` | AgentPackage -> JSON -> parseable |
| 6 | `error_undefined_capability` | USES cap_no_existe -> error |
| 7 | `error_duplicate_step_id` | Dos STEP login -> error |
| 8 | `error_invalid_risk` | RISK EXTREME -> error |

## No entra en v0.12

- Meta-agente generativo, eso es v0.13.
- Templates y herencia compleja.
- Condicionales en OPL, ya estan en runtime.
- Simulation integration, ya esta en v0.11.
- IDE / syntax highlighting.
- Formatter / linter.

## Dependencias a agregar

```toml
# Parser combinators
nom = "7.1"
# O alternativa manual con regex + iteradores
# Decision: nom para robustez, o manual para zero-deps
```

## Decision pendiente

`nom` vs parser manual:

- `nom`: robusto, combinators probados, learning curve.
- Manual: zero deps, mas codigo, mas bugs potenciales.

Recomendacion: `nom` para v1.0, refactor a manual si deps son problema.

## Checkpoint de diseno

| Item | Estado |
|------|--------|
| Gramatica EBNF | OK |
| Estructuras AST | OK |
| Transpilador | OK |
| Invariantes | OK |
| Tests planificados | OK |
| Scope negativo | OK |
| Decision pendiente (nom) | OK |
