use nu_plugin::{
    serve_plugin, EngineInterface, EvaluatedCall, LabeledError, MsgPackSerializer, Plugin,
    PluginCommand, SimplePluginCommand,
};
use nu_protocol::{
    ast::PathMember, Category, PluginExample, PluginSignature, Record, ShellError, Span, Spanned,
    SyntaxShape, Value,
};
use serde_json::Value as SerdeJsonValue;
use serde_json_path::JsonPath;

struct JsonPathPlugin;

impl Plugin for JsonPathPlugin {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(NuJsonPath)]
    }
}

// json path examples
// https://www.ietf.org/archive/id/draft-ietf-jsonpath-base-10.html#section-1.5
// json path docs
// https://docs.rs/serde_json_path/0.3.1/serde_json_path/
// json path repo
// https://github.com/hiltontj/serde_json_path
// serde json path grammar
// https://github.com/hiltontj/serde_json_path/blob/main/grammar.abnf

struct NuJsonPath;

impl SimplePluginCommand for NuJsonPath {
    type Plugin = JsonPathPlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("json path")
            .usage("View json path results")
            .required("query", SyntaxShape::String, "json path query")
            .category(Category::Experimental)
            .plugin_examples(vec![PluginExample {
                description: "List the authors of all books in the store".into(),
                example: "open -r test.json | json path '$.store.book[*].author'".into(),
                result: None,
            }])
    }

    fn run(
        &self,
        _config: &JsonPathPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let json_query: Option<Spanned<String>> = call.opt(0)?;
        let span = call.head;
        let input_span = &input.span();

        let json_path_results = match input {
            Value::String { val, .. } => perform_json_path_query(val, &json_query, input_span)?,
            Value::Record { val: _vals, .. } => {
                let json_value = value_to_json_value(input)?;
                let raw = serde_json::to_string(&json_value).unwrap();
                perform_json_path_query(&raw, &json_query, input_span)?
            }
            v => {
                return Err(LabeledError {
                    label: "Expected some input from pipeline".into(),
                    msg: format!("requires some input, got {}", v.get_type()),
                    span: Some(call.head),
                });
            }
        };

        let ret_list = Value::list(json_path_results, span);

        Ok(ret_list)
    }
}

fn perform_json_path_query(
    input: &str,
    json_query: &Option<Spanned<String>>,
    span: &nu_protocol::Span,
) -> Result<Vec<Value>, LabeledError> {
    let serde_json: SerdeJsonValue = serde_json::from_str(input).map_err(|e| LabeledError {
        label: "Error parsing json".into(),
        msg: e.to_string(),
        span: Some(*span),
    })?;

    let query = match &json_query.as_ref() {
        Some(p) => &p.item,
        None => {
            return Err(LabeledError {
                label: "Error parsing json query string".into(),
                msg: "No json path query provided".to_string(),
                span: Some(*span),
            })
        }
    };

    let path = JsonPath::parse(query).map_err(|e| LabeledError {
        label: "Error parsing json query".into(),
        msg: e.to_string(),
        span: Some(*span),
    })?;

    Ok(path
        .query(&serde_json)
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

            Value::list(v, span)
        }
        SerdeJsonValue::Bool(b) => Value::bool(*b, span),
        SerdeJsonValue::Number(f) => {
            if f.is_f64() {
                Value::float(f.as_f64().unwrap(), span)
            } else {
                Value::int(f.as_i64().unwrap(), span)
            }
        }
        SerdeJsonValue::Null => Value::nothing(span),
        SerdeJsonValue::Object(k) => {
            let mut rec = Record::new();
            for (k, v) in k {
                rec.push(k.clone(), convert_sjson_to_value(v, span));
            }
            Value::record(rec, span)
        }
        SerdeJsonValue::String(s) => Value::string(s.clone(), span),
    }
}

pub fn value_to_json_value(v: &Value) -> Result<SerdeJsonValue, LabeledError> {
    let val_span = v.span();
    Ok(match v {
        Value::Bool { val, .. } => SerdeJsonValue::Bool(*val),
        Value::Filesize { val, .. } => SerdeJsonValue::Number((*val).into()),
        Value::Duration { val, .. } => SerdeJsonValue::Number((*val).into()),
        Value::Date { val, .. } => SerdeJsonValue::String(val.to_string()),
        Value::Float { val, .. } => {
            SerdeJsonValue::Number(match serde_json::Number::from_f64(*val).ok_or(0.0) {
                Ok(n) => n,
                Err(e) => {
                    return Err(LabeledError {
                        label: format!("Error converting value: {val} to f64"),
                        msg: format!("Error converting {e}").to_string(),
                        span: Some(val_span),
                    })
                }
            })
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
        Value::Error { error, .. } => {
            return Err(LabeledError {
                label: format!("Error found: {error}"),
                msg: "Error found".to_string(),
                span: Some(v.span()),
            })
        }
        Value::Closure { .. } | Value::Block { .. } | Value::Range { .. } => SerdeJsonValue::Null,
        // | Value::MatchPattern { .. } => SerdeJsonValue::Null,
        Value::Binary { val, .. } => SerdeJsonValue::Array(
            val.iter()
                .map(|x| SerdeJsonValue::Number((*x as u64).into()))
                .collect(),
        ),
        Value::Record { val, .. } => {
            let mut m = serde_json::Map::new();
            val.iter().for_each(|(k, v)| {
                m.insert(k.clone(), value_to_json_value(v).unwrap());
            });
            SerdeJsonValue::Object(m)
        }
        Value::LazyRecord { val, .. } => {
            let collected = val.collect()?;
            value_to_json_value(&collected)?
        }
        Value::CustomValue { val, .. } => {
            let collected = val.to_base_value(val_span)?;
            value_to_json_value(&collected)?
        }
        Value::Glob {
            val: _val,
            no_expand: _no_expand,
            ..
        } => todo!(),
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
    serve_plugin(&JsonPathPlugin, MsgPackSerializer);
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value as SerdeJsonValue};
    use serde_json_path::JsonPath;

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
        let path = JsonPath::parse("$.store.book[*].author").unwrap();
        let nodes = path.query(&value).all();
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
        let path = JsonPath::parse("$..author").unwrap();
        let nodes = path.query(&value).all();
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
        let path = JsonPath::parse("$.store.*").unwrap();
        let nodes = path.query(&value).all();
        assert_eq!(nodes.len(), 2);
        assert!(nodes
            .iter()
            .any(|&node| node == value.pointer("/store/book").unwrap()));
    }

    #[test]
    fn spec_example_4() {
        let value = spec_example_json();
        let path = JsonPath::parse("$.store..price").unwrap();
        let nodes = path.query(&value).all();
        assert_eq!(nodes, vec![399., 8.95, 12.99, 8.99, 22.99]);
    }

    #[test]
    fn spec_example_5() {
        let value = spec_example_json();
        let path = JsonPath::parse("$..book[2]").unwrap();
        let node = path.query(&value).at_most_one().unwrap();
        assert!(node.is_some());
        assert_eq!(node, value.pointer("/store/book/2"));
    }

    #[test]
    fn spec_example_6() {
        let value = spec_example_json();
        let path = JsonPath::parse("$..book[-1]").unwrap();
        let node = path.query(&value).at_most_one().unwrap();
        assert!(node.is_some());
        assert_eq!(node, value.pointer("/store/book/3"));
    }

    #[test]
    fn spec_example_7() {
        let value = spec_example_json();
        {
            let path = JsonPath::parse("$..book[0,1]").unwrap();
            let nodes = path.query(&value).all();
            assert_eq!(nodes.len(), 2);
        }
        {
            let path = JsonPath::parse("$..book[:2]").unwrap();
            let nodes = path.query(&value).all();
            assert_eq!(nodes.len(), 2);
        }
    }

    #[test]
    fn spec_example_8() {
        let value = spec_example_json();
        let path = JsonPath::parse("$..book[?(@.isbn)]").unwrap();
        let nodes = path.query(&value);
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn spec_example_9() {
        let value = spec_example_json();
        let path = JsonPath::parse("$..book[?(@.price<10)]").unwrap();
        let nodes = path.query(&value);
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn spec_example_10() {
        let value = spec_example_json();
        let path = JsonPath::parse("$..*").unwrap();
        let nodes = path.query(&value);
        assert_eq!(nodes.len(), 27);
    }
}
