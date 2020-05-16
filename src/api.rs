use crate::model::{Span, Trace};
use std::{
    collections::HashMap,
    time::{Duration, UNIX_EPOCH}
};
use serde::Serialize;

fn fill_meta(span: &Span, env: Option<String>) -> HashMap<String, String> {
    let mut meta = HashMap::new();
    if let Some(env) = env {
        meta.insert("env".to_string(), env);
    }

    if let Some(http) = &span.http {
        meta.insert("http.status_code".to_string(), http.status_code.clone());
        meta.insert("http.method".to_string(), http.method.clone());
        meta.insert("http.url".to_string(), http.url.clone());
    }
    if let Some(error) = &span.error {
        meta.insert("error.type".to_string(), error.r#type.clone());
        meta.insert("error.msg".to_string(), error.msg.clone());
        meta.insert("error.stack".to_string(), error.stack.clone());
    }
    if let Some(sql) = &span.sql {
        meta.insert("sql.query".to_string(), sql.query.clone());
        meta.insert("sql.rows".to_string(), sql.rows.clone());
        meta.insert("sql.db".to_string(), sql.db.clone());
    }
    for (key, value) in &span.tags {
        meta.insert(key.to_string(), value.to_string());
    }
    meta
}

fn fill_metrics(priority: u32) -> HashMap<String, f64> {
    let mut metrics = HashMap::new();
    metrics.insert("_sampling_priority_v1".to_string(), f64::from(priority));
    metrics
}

fn duration_to_nanos(duration: Duration) -> u64 {
    duration.as_secs() * 1_000_000_000 + duration.subsec_nanos() as u64
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct RawTrace(Vec<RawSpan>);

impl RawTrace {
    pub fn from_trace(trace: &Trace, service: &String, env: &Option<String>) -> RawTrace {
        RawTrace(
            trace.spans.iter().map(|span| RawSpan::from_span(span, trace, service, env)).collect()
        )
    }
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct RawSpan {
    service: String,
    name: String,
    resource: String,
    trace_id: u64,
    span_id: u64,
    parent_id: Option<u64>,
    start: u64,
    duration: u64,
    error: i32,
    meta: HashMap<String, String>,
    metrics: HashMap<String, f64>,
    r#type: String,
}

impl RawSpan {
    pub fn from_span(span: &Span, trace: &Trace, service: &String, env: &Option<String>) -> RawSpan {
        RawSpan {
            service: service.clone(),
            trace_id: trace.id,
            span_id: span.id,
            name: span.name.clone(),
            resource: span.resource.clone(),
            parent_id: span.parent_id,
            start: duration_to_nanos(span.start.duration_since(UNIX_EPOCH).unwrap()),
            duration: duration_to_nanos(span.duration),
            error: if span.error.is_some() { 1 } else { 0 },
            r#type: span.r#type.clone(),
            meta: fill_meta(&span, env.clone()),
            metrics: fill_metrics(trace.priority),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::Config;

    use super::*;
    use crate::model::HttpInfo;
    use std::time::SystemTime;

    use rand::Rng;

    #[test]
    fn test_map_to_raw_spans() {
        let config = Config {
            service: String::from("service_name"),
            env: Some(String::from("staging")),
            ..Default::default()
        };
        let mut rng = rand::thread_rng();
        let trace = Trace {
            id: rng.gen::<u64>(),
            priority: 1,
            spans: vec![Span {
                id: rng.gen::<u64>(),
                name: String::from("request"),
                resource: String::from("/home/v3"),
                r#type: String::from("web"),
                start: SystemTime::now(),
                duration: Duration::from_secs(2),
                parent_id: None,
                http: Some(HttpInfo {
                    url: String::from("/home/v3/2?trace=true"),
                    method: String::from("GET"),
                    status_code: String::from("200"),
                }),
                error: None,
                sql: None,
                tags: HashMap::new(),
            }],
        };

        let mut expected = Vec::new();
        for span in &trace.spans {
            let mut meta: HashMap<String, String> = HashMap::new();
            meta.insert("env".to_string(), config.env.clone().unwrap());
            if let Some(http) = &span.http {
                meta.insert("http.url".to_string(), http.url.clone());
                meta.insert("http.method".to_string(), http.method.clone());
                meta.insert("http.status_code".to_string(), http.status_code.clone());
            }

            let mut metrics = HashMap::new();
            metrics.insert(
                "_sampling_priority_v1".to_string(),
                f64::from(trace.priority),
            );

            expected.push(RawSpan {
                trace_id: trace.id,
                span_id: span.id,
                parent_id: span.parent_id,
                name: span.name.clone(),
                resource: span.resource.clone(),
                service: config.service.clone(),
                r#type: span.r#type.clone(),
                start: duration_to_nanos(span.start.duration_since(UNIX_EPOCH).unwrap()),
                duration: duration_to_nanos(span.duration),
                error: 0,
                meta: meta,
                metrics: metrics,
            });
        }
        let raw_spans = RawTrace::from_trace(&trace, &config.service, &config.env);

        assert_eq!(raw_spans.0, expected);
    }
}
