use std::collections::HashMap;

use rocket::fairing::Fairing;
use rocket_dyn_templates::tera::{self, Error, Result, Value};
use rocket_dyn_templates::Template;

pub fn custom() -> impl Fairing {
    Template::custom(|engines| engines.tera.register_function("contains", contains))
}

fn contains(args: &HashMap<String, Value>) -> Result<Value> {
    let err = || {
        Err(Error::msg(format!(
            "Invalid arguments, expected `values=[string], value=string`, given: {args:?}"
        )))
    };

    match (args.get("values"), args.get("value")) {
        (Some(values), Some(value)) => match (
            tera::from_value::<Vec<String>>(values.clone()),
            tera::from_value::<String>(value.clone()),
        ) {
            (Ok(values), Ok(value)) => {
                Ok(values.iter().any(|v| v.eq_ignore_ascii_case(&value)).into())
            }
            _ => err(),
        },
        _ => err(),
    }
}
