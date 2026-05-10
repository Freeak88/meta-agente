use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{char, multispace0, multispace1},
    combinator::{map, value},
    multi::separated_list0,
    sequence::{delimited, preceded},
    IResult,
};

use crate::dsl::{AgentDecl, FallbackDecl, OplAst, OplValue, StepDecl};

#[derive(Debug, Clone)]
enum Statement {
    Step(StepDecl),
    Fallback(FallbackDecl),
}

fn identifier(input: &str) -> IResult<&str, String> {
    let (input, id) = take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '-')(input)?;
    Ok((input, id.to_string()))
}

fn string_literal(input: &str) -> IResult<&str, String> {
    let (input, _) = char('"')(input)?;
    let (input, content) = take_until("\"")(input)?;
    let (input, _) = char('"')(input)?;
    Ok((input, content.to_string()))
}

fn number_literal(input: &str) -> IResult<&str, f64> {
    let (input, number) = take_while1(|c: char| c.is_ascii_digit() || c == '.' || c == '-')(input)?;
    let parsed = number.parse::<f64>().map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
    })?;
    Ok((input, parsed))
}

fn boolean_literal(input: &str) -> IResult<&str, bool> {
    alt((value(true, tag("true")), value(false, tag("false"))))(input)
}

fn opl_value(input: &str) -> IResult<&str, OplValue> {
    alt((
        map(string_literal, OplValue::String),
        map(boolean_literal, OplValue::Bool),
        map(number_literal, OplValue::Number),
        map(list_literal, OplValue::List),
        map(object_literal, OplValue::Object),
        map(identifier, OplValue::Identifier),
    ))(input)
}

fn list_literal(input: &str) -> IResult<&str, Vec<OplValue>> {
    delimited(
        delimited(multispace0, char('['), multispace0),
        separated_list0(delimited(multispace0, char(','), multispace0), opl_value),
        delimited(multispace0, char(']'), multispace0),
    )(input)
}

fn property(input: &str) -> IResult<&str, (String, OplValue)> {
    let (input, key) = delimited(multispace0, identifier, multispace0)(input)?;
    let (input, _) = char('=')(input)?;
    let (input, value) = delimited(multispace0, opl_value, multispace0)(input)?;
    Ok((input, (key, value)))
}

fn object_literal(input: &str) -> IResult<&str, HashMap<String, OplValue>> {
    let (input, properties) = delimited(
        delimited(multispace0, char('{'), multispace0),
        separated_list0(delimited(multispace0, char(','), multispace0), property),
        delimited(multispace0, char('}'), multispace0),
    )(input)?;
    Ok((input, properties.into_iter().collect()))
}

fn property_list(input: &str) -> IResult<&str, HashMap<String, OplValue>> {
    let (input, properties) =
        separated_list0(delimited(multispace0, char(','), multispace0), property)(input)?;
    Ok((input, properties.into_iter().collect()))
}

fn create_agent_header(input: &str) -> IResult<&str, (String, String)> {
    let (input, _) = delimited(multispace0, tag("CREATE"), multispace1)(input)?;
    let (input, _) = tag("AGENT")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, id) = identifier(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag("FOR")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, mission) = string_literal(input)?;
    Ok((input, (id, mission)))
}

fn step_header(input: &str) -> IResult<&str, (String, String)> {
    let (input, _) = delimited(multispace0, tag("STEP"), multispace1)(input)?;
    let (input, id) = identifier(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag("USES")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, capability) = identifier(input)?;
    Ok((input, (id, capability)))
}

fn step_clause(input: &str) -> IResult<&str, (&str, OplValue)> {
    let risk = map(
        preceded(delimited(multispace0, tag("RISK"), multispace1), identifier),
        |risk| ("RISK", OplValue::Identifier(risk)),
    );
    let hitl = map(
        preceded(
            delimited(multispace0, tag("HITL"), multispace1),
            boolean_literal,
        ),
        |hitl| ("HITL", OplValue::Bool(hitl)),
    );
    let input_clause = map(
        preceded(delimited(multispace0, tag("INPUT"), multispace1), opl_value),
        |input| ("INPUT", input),
    );
    let validate = map(
        preceded(
            delimited(multispace0, tag("VALIDATE"), multispace1),
            list_literal,
        ),
        |values| ("VALIDATE", OplValue::List(values)),
    );

    alt((risk, hitl, input_clause, validate))(input)
}

fn fallback_header(input: &str) -> IResult<&str, (String, String)> {
    let (input, _) = delimited(multispace0, tag("ON"), multispace1)(input)?;
    let (input, condition) = identifier(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag("FALLBACK")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag("TO")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, target) = identifier(input)?;
    Ok((input, (condition, target)))
}

fn fallback_clause(input: &str) -> IResult<&str, (&str, OplValue)> {
    let retry = map(
        preceded(
            delimited(multispace0, tag("RETRY"), multispace1),
            number_literal,
        ),
        |retry| ("RETRY", OplValue::Number(retry)),
    );
    let backoff = map(
        preceded(
            delimited(multispace0, tag("BACKOFF"), multispace1),
            identifier,
        ),
        |backoff| ("BACKOFF", OplValue::Identifier(backoff)),
    );

    alt((retry, backoff))(input)
}

fn section_blocks(input: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();

    for raw_line in input.lines() {
        let line = raw_line.trim_end();
        let starts_declaration = line.trim_start().starts_with("CREATE ")
            || line.trim_start().starts_with("STEP ")
            || line.trim_start().starts_with("ON ");

        if starts_declaration && !current.is_empty() {
            blocks.push(current.join("\n"));
            current.clear();
        }

        if !line.trim().is_empty() {
            current.push(line.to_string());
        }
    }

    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }

    blocks
}

fn parse_agent_block(block: &str) -> Result<(String, String, HashMap<String, OplValue>), String> {
    let lines: Vec<&str> = block.lines().collect();
    if lines.len() < 2 {
        return Err("missing CREATE AGENT header".to_string());
    }
    let header = lines[0..2].join("\n");
    let (remaining, (id, mission)) =
        create_agent_header(&header).map_err(|err| format!("{err:?}"))?;
    if !remaining.trim().is_empty() {
        return Err(format!("unparsed agent header: {remaining}"));
    }

    let mut properties = HashMap::new();
    if let Some(with_index) = lines.iter().position(|line| line.trim() == "WITH") {
        let property_text = lines[(with_index + 1)..].join("\n");
        let (remaining, parsed) = property_list(&property_text)
            .map_err(|err| format!("property parse error: {err:?}"))?;
        if !remaining.trim().is_empty() {
            return Err(format!("unparsed properties: {remaining}"));
        }
        properties = parsed;
    }

    Ok((id, mission, properties))
}

fn parse_step_block(block: &str) -> Result<StepDecl, String> {
    let lines: Vec<&str> = block.lines().collect();
    if lines.len() < 2 {
        return Err("missing STEP header".to_string());
    }
    let header = lines[0..2].join("\n");
    let (remaining, (id, capability)) = step_header(&header).map_err(|err| format!("{err:?}"))?;
    if !remaining.trim().is_empty() {
        return Err(format!("unparsed step header: {remaining}"));
    }

    let mut risk = None;
    let mut hitl = None;
    let mut input = None;
    let mut validate = Vec::new();

    for line in lines.into_iter().skip(2) {
        let (remaining, (name, value)) =
            step_clause(line).map_err(|_| format!("unparsed input remaining: {line}"))?;
        if !remaining.trim().is_empty() {
            return Err(format!("unparsed step clause: {remaining}"));
        }
        match (name, value) {
            ("RISK", OplValue::Identifier(value)) => risk = Some(value),
            ("HITL", OplValue::Bool(value)) => hitl = Some(value),
            ("INPUT", value) => input = Some(value),
            ("VALIDATE", OplValue::List(values)) => {
                validate = values
                    .into_iter()
                    .map(|value| match value {
                        OplValue::String(value) | OplValue::Identifier(value) => Ok(value),
                        _ => Err("VALIDATE accepts only identifiers or strings".to_string()),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
            }
            _ => return Err(format!("unsupported step clause: {line}")),
        }
    }

    Ok(StepDecl {
        id,
        capability,
        properties: HashMap::new(),
        risk,
        hitl,
        input,
        validate,
    })
}

fn parse_fallback_block(block: &str) -> Result<FallbackDecl, String> {
    let lines: Vec<&str> = block.lines().collect();
    if lines.len() < 2 {
        return Err("missing ON header".to_string());
    }
    let header = lines[0..2].join("\n");
    let (remaining, (condition, target)) =
        fallback_header(&header).map_err(|err| format!("{err:?}"))?;
    if !remaining.trim().is_empty() {
        return Err(format!("unparsed fallback header: {remaining}"));
    }

    let mut retry = None;
    let mut backoff = None;

    for line in lines.into_iter().skip(2) {
        let (remaining, (name, value)) =
            fallback_clause(line).map_err(|_| format!("unparsed input remaining: {line}"))?;
        if !remaining.trim().is_empty() {
            return Err(format!("unparsed fallback clause: {remaining}"));
        }
        match (name, value) {
            ("RETRY", OplValue::Number(value)) => retry = Some(value as u32),
            ("BACKOFF", OplValue::Identifier(value)) => backoff = Some(value),
            _ => return Err(format!("unsupported fallback clause: {line}")),
        }
    }

    Ok(FallbackDecl {
        condition,
        target,
        retry,
        backoff,
    })
}

pub fn parse_opl(input: &str) -> Result<OplAst, String> {
    let blocks = section_blocks(input);
    let agent_block = blocks
        .iter()
        .find(|block| block.trim_start().starts_with("CREATE "))
        .ok_or_else(|| "parse error: missing CREATE AGENT declaration".to_string())?;

    let (id, mission, properties) = parse_agent_block(agent_block)?;
    let mut statements = Vec::new();

    for block in blocks
        .iter()
        .filter(|block| !block.trim_start().starts_with("CREATE "))
    {
        if block.trim_start().starts_with("STEP ") {
            statements.push(Statement::Step(parse_step_block(block)?));
        } else if block.trim_start().starts_with("ON ") {
            statements.push(Statement::Fallback(parse_fallback_block(block)?));
        } else {
            return Err(format!("unparsed input remaining: {block}"));
        }
    }

    let mut steps = Vec::new();
    let mut fallbacks = Vec::new();
    for statement in statements {
        match statement {
            Statement::Step(step) => steps.push(step),
            Statement::Fallback(fallback) => fallbacks.push(fallback),
        }
    }

    Ok(OplAst {
        agent: AgentDecl {
            id,
            mission,
            properties,
            steps,
            fallbacks,
        },
    })
}
