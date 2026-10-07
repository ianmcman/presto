#![allow(dead_code)]

use std::collections::BTreeSet;

pub const FORBIDDEN: &[&str] = &[
    "token",
    "auth",
    "authorization",
    "bearer",
    "jwt",
    "secret",
    "cookie",
    "password",
    "credential",
];

// Exact-match exceptions (D-15). Each is a state name, not a credential.
pub const ALLOW: &[&str] = &[
    "auth",         // Event::Auth variant tag; payload is AuthState only
    "auth_expired", // ErrorKind::AuthExpired and FaultSpec::AuthExpired tags
];

/// Collect property keys, `const` strings and `enum` strings from a JSON schema.
pub fn names(v: &serde_json::Value, out: &mut BTreeSet<String>) {
    match v {
        serde_json::Value::Object(m) => {
            for (k, val) in m {
                match (k.as_str(), val) {
                    ("properties", serde_json::Value::Object(p)) => {
                        out.extend(p.keys().cloned());
                    }
                    ("const", serde_json::Value::String(s)) => {
                        out.insert(s.clone());
                    }
                    ("enum", serde_json::Value::Array(a)) => {
                        out.extend(a.iter().filter_map(|x| x.as_str().map(String::from)));
                    }
                    _ => {}
                }
                names(val, out);
            }
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| names(x, out)),
        _ => {}
    }
}

pub fn schema_names<T: schemars::JsonSchema>() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    names(&serde_json::to_value(schemars::schema_for!(T)).unwrap(), &mut out);
    out
}

pub fn is_forbidden(name: &str) -> bool {
    !ALLOW.contains(&name)
        && name
            .split(['_', '-'])
            .any(|seg| FORBIDDEN.contains(&seg.to_ascii_lowercase().as_str()))
}
