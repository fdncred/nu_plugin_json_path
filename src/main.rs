use nu_plugin::{serve_plugin, EvaluatedCall, LabeledError, MsgPackSerializer, Plugin};
use nu_protocol::{
    ast::PathMember, Category, PluginExample, PluginSignature, ShellError, Span, Spanned,
    SyntaxShape, Value,
};
use serde_json::Value as SerdeJsonValue;
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
                description: "List the authors of all books in the store".into(),
                example: "open -r test.json | json path '$.store.book[*].author'".into(),
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

        let json_path_results = match input {
            Value::String { val, span } => perform_json_path_query(val, &param, span)?,
            Value::Record {
                cols: _cols,
                vals: _vals,
                span,
            } => {
                let json_value = value_to_json_value(input)?;
                let raw = serde_json::to_string(&json_value).unwrap();
                perform_json_path_query(&raw, &param, span)?
            }
            v => {
                return Err(LabeledError {
                    label: "Expected some input from pipeline".into(),
                    msg: format!("requires some input, got {}", v.get_type()),
                    span: Some(call.head),
                });
            }
        };

        let ret_list = Value::List {
            vals: json_path_results,
            span,
        };

        Ok(ret_list)
    }
}

fn perform_json_path_query(
    input: &str,
    param: &Option<Spanned<String>>,
    span: &nu_protocol::Span,
) -> Result<Vec<Value>, LabeledError> {
    let serde_json: SerdeJsonValue = serde_json::from_str(input).unwrap();
    Ok(serde_json
        .json_path(&param.as_ref().unwrap().item)
        .unwrap()
        .all()
        .into_iter()
        .map(|v| convert_sjson_to_value(v, *span))
        .collect())
}

pub fn convert_sjson_to_value(value: &SerdeJsonValue, span: Span) -> Value {
    match value {
        SerdeJsonValue::Array(array) => {
            let v: Vec<Value> = array
                .iter()
                .map(|x| convert_sjson_to_value(x, span))
                .collect();

            Value::List { vals: v, span }
        }
        SerdeJsonValue::Bool(b) => Value::Bool { val: *b, span },
        SerdeJsonValue::Number(f) => {
            if f.is_f64() {
                Value::Float {
                    val: f.as_f64().unwrap(),
                    span,
                }
            } else {
                Value::Int {
                    val: f.as_i64().unwrap(),
                    span,
                }
            }
        }
        SerdeJsonValue::Null => Value::Nothing { span },
        SerdeJsonValue::Object(k) => {
            let mut cols = vec![];
            let mut vals = vec![];

            for item in k {
                cols.push(item.0.clone());
                vals.push(convert_sjson_to_value(item.1, span));
            }

            Value::Record { cols, vals, span }
        }
        SerdeJsonValue::String(s) => Value::String {
            val: s.clone(),
            span,
        },
    }
}

pub fn value_to_json_value(v: &Value) -> Result<SerdeJsonValue, ShellError> {
    Ok(match v {
        Value::Bool { val, .. } => SerdeJsonValue::Bool(*val),
        Value::Filesize { val, .. } => SerdeJsonValue::Number((*val).into()),
        Value::Duration { val, .. } => SerdeJsonValue::Number((*val).into()),
        Value::Date { val, .. } => SerdeJsonValue::String(val.to_string()),
        Value::Float { val, span } => {
            SerdeJsonValue::Number(serde_json::Number::from_f64(*val).ok_or(0.0).map_err(|_| {
                //FIXME: This error needs to be more descriptive
                ShellError::CantConvert(
                    "cant convert".to_string(),
                    "something else".to_string(),
                    *span,
                    None,
                )
            })?)
        }
        Value::Int { val, .. } => SerdeJsonValue::Number((*val).into()),
        Value::Nothing { .. } => SerdeJsonValue::Null,
        Value::String { val, .. } => SerdeJsonValue::String(val.to_string()),
        Value::CellPath { val, .. } => SerdeJsonValue::Array(
            val.members
                .iter()
                .map(|x| match &x {
                    PathMember::String { val, .. } => Ok(SerdeJsonValue::String(val.clone())),
                    PathMember::Int { val, .. } => Ok(SerdeJsonValue::Number((*val as u64).into())),
                })
                .collect::<Result<Vec<SerdeJsonValue>, ShellError>>()?,
        ),

        Value::List { vals, .. } => SerdeJsonValue::Array(json_list(vals)?),
        Value::Error { error } => return Err(error.clone()),
        Value::Closure { .. } | Value::Block { .. } | Value::Range { .. } => SerdeJsonValue::Null,
        Value::Binary { val, .. } => SerdeJsonValue::Array(
            val.iter()
                .map(|x| SerdeJsonValue::Number((*x as u64).into()))
                .collect(),
        ),
        Value::Record { cols, vals, .. } => {
            let mut m = serde_json::Map::new();
            for (k, v) in cols.iter().zip(vals) {
                m.insert(k.clone(), value_to_json_value(v)?);
            }
            SerdeJsonValue::Object(m)
        }
        Value::LazyRecord { val, .. } => {
            let collected = val.collect()?;
            value_to_json_value(&collected)?
        }
        Value::CustomValue { val, span } => {
            serde_json::from_str(&val.value_string()).map_err(|_| {
                //FIXME: This error needs to be more descriptive
                ShellError::CantConvert(
                    "cant convert".to_string(),
                    "something else".to_string(),
                    *span,
                    None,
                )
            })?
        }
    })
}

fn json_list(input: &[Value]) -> Result<Vec<SerdeJsonValue>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(value_to_json_value(value)?);
    }

    Ok(out)
}

fn main() {
    serve_plugin(&mut JsonPath::new(), MsgPackSerializer);
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value as SerdeJsonValue;
    use serde_json_path::JsonPathExt;

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
}
