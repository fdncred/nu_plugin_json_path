mod json_path;
use nu_json::Value as NuJsonValue;
use nu_plugin::{serve_plugin, EvaluatedCall, LabeledError, MsgPackSerializer, Plugin};
use nu_protocol::{
    ast::PathMember, Category, PluginExample, PluginSignature, ShellError, Spanned, SyntaxShape,
    Value,
};
use serde_json::{json, Value as SerdeJsonValue};
use serde_json_path::JsonPathExt;

// json path examples
// https://www.ietf.org/archive/id/draft-ietf-jsonpath-base-10.html#section-1.5
// json path docs
// https://docs.rs/serde_json_path/0.3.1/serde_json_path/
// json path repo
// https://github.com/hiltontj/serde_json_path
// serde json path grammar
// https://github.com/hiltontj/serde_json_path/blob/main/grammar.abnf

struct JsonPath;

impl JsonPath {
    fn new() -> Self {
        Self {}
    }
}

impl Plugin for JsonPath {
    fn signature(&self) -> Vec<PluginSignature> {
        vec![PluginSignature::build("json path")
            .usage("View json path results")
            .required("query", SyntaxShape::String, "json path query")
            .category(Category::Experimental)
            .plugin_examples(vec![PluginExample {
                description: "This is the example descripion".into(),
                example: "some pipeline involving json_path".into(),
                result: None,
            }])]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        assert_eq!(name, "json path");
        let param: Option<Spanned<String>> = call.opt(0)?;

        let span = call.head;
        // let value = input.into_value(span);
        let json_value = value_to_json_value(&input)?;
        let serde_json: SerdeJsonValue = serde_json::from_str(&input.as_string()?).unwrap();

        let json_path_results = match input {
            Value::String { val, span } => {
                // crate::json_path::json_path_do_something(param, val, *span)?
                // json_value.json_path(&param.unwrap().item).unwrap().all()
                serde_json
                    .json_path(&param.unwrap().item)
                    .unwrap()
                    .all()
                    .into_iter()
                    .map(|v| Value::String {
                        val: v.to_string(),
                        span: *span,
                    })
                    .collect()
            }
            v => {
                return Err(LabeledError {
                    label: "Expected something from pipeline".into(),
                    msg: format!("requires some input, got {}", v.get_type()),
                    span: Some(call.head),
                });
            }
        };

        let ret_list = Value::List {
            vals: json_path_results,
            span,
        };

        let sample_json = spec_example_json();
        let example1 = sample_json
            .json_path("$.store.book[*].author")
            .unwrap()
            .all();
        eprintln!("example1: {:?}", example1);
        let example2 = sample_json.json_path("$..author").unwrap().all();
        eprintln!("example2: {:?}", example2);
        let example3 = sample_json.json_path("$.store.*").unwrap().all();
        eprintln!("example3: {:?}", example3);
        let example4 = sample_json.json_path("$.store..price").unwrap().all();
        eprintln!("example4: {:?}", example4);
        let example5 = sample_json.json_path("$..book[2]").unwrap().all();
        eprintln!("example5: {:?}", example5);
        let example6 = sample_json.json_path("$..book[-1]").unwrap().all();
        eprintln!("example6: {:?}", example6);
        let example7 = sample_json.json_path("$..book[0,1]").unwrap().all();
        eprintln!("example7: {:?}", example7);
        let example8 = sample_json.json_path("$..book[:2]").unwrap().all();
        eprintln!("example8: {:?}", example8);
        let example9 = sample_json.json_path("$..book[?(@.isbn)]").unwrap().all();
        eprintln!("example9: {:?}", example9);
        let example10 = sample_json
            .json_path("$..book[?(@.price<10)]")
            .unwrap()
            .all();
        eprintln!("example10: {:?}", example10);
        let example11 = sample_json.json_path("$..*").unwrap().all();
        eprintln!("example11: {:?}", example11);
        let example12 = sample_json
            .json_path("$..book[?(@.price<30 && @.price>10)]")
            .unwrap()
            .all();
        eprintln!("example12: {:?}", example12);

        Ok(ret_list)
    }
}

pub fn value_to_json_value(v: &Value) -> Result<NuJsonValue, ShellError> {
    Ok(match v {
        Value::Bool { val, .. } => NuJsonValue::Bool(*val),
        Value::Filesize { val, .. } => NuJsonValue::I64(*val),
        Value::Duration { val, .. } => NuJsonValue::I64(*val),
        Value::Date { val, .. } => NuJsonValue::String(val.to_string()),
        Value::Float { val, .. } => NuJsonValue::F64(*val),
        Value::Int { val, .. } => NuJsonValue::I64(*val),
        Value::Nothing { .. } => NuJsonValue::Null,
        Value::String { val, .. } => NuJsonValue::String(val.to_string()),
        Value::CellPath { val, .. } => NuJsonValue::Array(
            val.members
                .iter()
                .map(|x| match &x {
                    PathMember::String { val, .. } => Ok(NuJsonValue::String(val.clone())),
                    PathMember::Int { val, .. } => Ok(NuJsonValue::U64(*val as u64)),
                })
                .collect::<Result<Vec<NuJsonValue>, ShellError>>()?,
        ),

        Value::List { vals, .. } => NuJsonValue::Array(json_list(vals)?),
        Value::Error { error } => return Err(error.clone()),
        Value::Closure { .. } | Value::Block { .. } | Value::Range { .. } => NuJsonValue::Null,
        Value::Binary { val, .. } => {
            NuJsonValue::Array(val.iter().map(|x| NuJsonValue::U64(*x as u64)).collect())
        }
        Value::Record { cols, vals, .. } => {
            let mut m = nu_json::Map::new();
            for (k, v) in cols.iter().zip(vals) {
                m.insert(k.clone(), value_to_json_value(v)?);
            }
            NuJsonValue::Object(m)
        }
        Value::LazyRecord { val, .. } => {
            let collected = val.collect()?;
            value_to_json_value(&collected)?
        }
        Value::CustomValue { val, .. } => val.to_json(),
    })
}

fn json_list(input: &[Value]) -> Result<Vec<NuJsonValue>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(value_to_json_value(value)?);
    }

    Ok(out)
}

fn main() {
    serve_plugin(&mut JsonPath::new(), MsgPackSerializer);
}

fn spec_example_json() -> SerdeJsonValue {
    json!({
        "store": {
            "book": [
                {
                    "category": "reference",
                    "author": "Nigel Rees",
                    "title": "Sayings of the Century",
                    "price": 8.95
                },
                {
                    "category": "fiction",
                    "author": "Evelyn Waugh",
                    "title": "Sword of Honour",
                    "price": 12.99
                },
                {
                    "category": "fiction",
                    "author": "Herman Melville",
                    "title": "Moby Dick",
                    "isbn": "0-553-21311-3",
                    "price": 8.99
                },
                {
                    "category": "fiction",
                    "author": "J. R. R. Tolkien",
                    "title": "The Lord of the Rings",
                    "isbn": "0-395-19395-8",
                    "price": 22.99
                }
            ],
            "bicycle": {
                "color": "red",
                "price": 399
            }
        }
    })
}

#[test]
fn spec_example_1() {
    let value = spec_example_json();
    let nodes = value.json_path("$.store.book[*].author").unwrap().all();
    assert_eq!(
        nodes,
        vec![
            "Nigel Rees",
            "Evelyn Waugh",
            "Herman Melville",
            "J. R. R. Tolkien"
        ]
    );
}

#[test]
fn spec_example_2() {
    let value = spec_example_json();
    let nodes = value.json_path("$..author").unwrap().all();
    assert_eq!(
        nodes,
        vec![
            "Nigel Rees",
            "Evelyn Waugh",
            "Herman Melville",
            "J. R. R. Tolkien"
        ]
    );
}

#[test]
fn spec_example_3() {
    let value = spec_example_json();
    let nodes = value.json_path("$.store.*").unwrap().all();
    assert_eq!(nodes.len(), 2);
    assert!(nodes
        .iter()
        .any(|&node| node == value.pointer("/store/book").unwrap()));
}

#[test]
fn spec_example_4() {
    let value = spec_example_json();
    let nodes = value.json_path("$.store..price").unwrap().all();
    assert_eq!(nodes, vec![399., 8.95, 12.99, 8.99, 22.99]);
}

#[test]
fn spec_example_5() {
    let value = spec_example_json();
    let q = value.json_path("$..book[2]").unwrap();
    let node = q.one().unwrap();
    assert_eq!(node, value.pointer("/store/book/2").unwrap());
}

#[test]
fn spec_example_6() {
    let value = spec_example_json();
    let q = value.json_path("$..book[-1]").unwrap();
    let node = q.one().unwrap();
    assert_eq!(node, value.pointer("/store/book/3").unwrap());
}

#[test]
fn spec_example_7() {
    let value = spec_example_json();
    {
        let q = value.json_path("$..book[0,1]").unwrap();
        assert_eq!(q.len(), 2);
    }
    {
        let q = value.json_path("$..book[:2]").unwrap();
        assert_eq!(q.len(), 2);
    }
}

#[test]
fn spec_example_8() {
    let value = spec_example_json();
    let q = value.json_path("$..book[?(@.isbn)]").unwrap();
    assert_eq!(q.len(), 2);
}

#[test]
fn spec_example_9() {
    let value = spec_example_json();
    let q = value.json_path("$..book[?(@.price<10)]").unwrap();
    assert_eq!(q.len(), 2);
}

#[test]
fn spec_example_10() {
    let value = spec_example_json();
    let q = value.json_path("$..*").unwrap();
    assert_eq!(q.len(), 27);
}
