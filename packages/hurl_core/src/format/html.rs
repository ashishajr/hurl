/*
 * Hurl (https://hurl.dev)
 * Copyright (C) 2025 Orange
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */
use std::fmt::Display;

use crate::ast::{
    Assert, Base64, Body, BooleanOption, Bytes, Capture, CertificateAttributeName, Comment, Cookie,
    CookieAttribute, CookiePath, CountOption, DurationOption, Entry, EntryOption, File,
    FilenameParam, FilenameValue, Filter, FilterValue, Hex, HurlFile, JsonValue, KeyValue,
    LineTerminator, Method, MultilineString, MultipartParam, NaturalOption, OptionKind,
    Placeholder, Predicate, PredicateFunc, PredicateFuncValue, PredicateValue, Query, QueryValue,
    Regex, RegexValue, Request, Response, Section, SectionValue, Status, Template,
    VariableDefinition, VariableValue, Version, Whitespace,
};
use crate::typing::{Count, ToSource};

/// Returns an HTML string of the Hurl file `hurl_file`.
///
/// If `standalone` is true, a complete HTML body with inline styling is returned.
/// Otherwise, a `<pre>` HTML tag is returned, without styling.
pub fn format(hurl_file: &HurlFile, standalone: bool) -> String {
    let mut fmt = HtmlFormatter::new();
    let body = fmt.fmt_hurl_file(hurl_file);
    if standalone {
        let css = include_str!("hurl.css");
        format!(
            r#"<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <title>Hurl File</title>
        <style>
{css}
        </style>
    </head>
    <body>
{body}
    </body>
</html>
"#
        )
    } else {
        body.to_string()
    }
}

pub fn hurl_css() -> String {
    include_str!("hurl.css").to_string()
}

/// A HTML formatter for Hurl content.
struct HtmlFormatter {
    buffer: String,
}

impl HtmlFormatter {
    pub fn new() -> Self {
        HtmlFormatter {
            buffer: String::new(),
        }
    }

    pub fn fmt_hurl_file(&mut self, hurl_file: &HurlFile) -> &str {
        self.buffer.clear();
        self.fmt_pre_open("language-hurl");
        hurl_file.entries.iter().for_each(|e| self.fmt_entry(e));
        self.fmt_lts(&hurl_file.line_terminators);
        self.fmt_pre_close();
        &self.buffer
    }

    fn fmt_pre_open(&mut self, class: &str) {
        self.buffer.push_str("<pre><code class=\"");
        self.buffer.push_str(class);
        self.buffer.push_str("\">");
    }

    fn fmt_pre_close(&mut self) {
        self.buffer.push_str("</code></pre>");
    }

    fn fmt_span_open(&mut self, class: &str) {
        self.buffer.push_str("<span class=\"");
        self.buffer.push_str(class);
        self.buffer.push_str("\">");
    }

    fn fmt_span_close(&mut self) {
        self.buffer.push_str("</span>");
    }

    fn fmt_span(&mut self, class: &str, value: &str) {
        self.buffer.push_str("<span class=\"");
        self.buffer.push_str(class);
        self.buffer.push_str("\">");
        self.buffer.push_str(value);
        self.buffer.push_str("</span>");
    }

    fn fmt_entry(&mut self, entry: &Entry) {
        self.fmt_span_open("entry");
        self.fmt_request(&entry.request);
        if let Some(response) = &entry.response {
            self.fmt_response(response);
        }
        self.fmt_span_close();
    }

    fn fmt_request(&mut self, request: &Request) {
        self.fmt_span_open("request");
        self.fmt_lts(&request.line_terminators);
        self.fmt_space(&request.space0);
        self.fmt_method(&request.method);
        self.fmt_space(&request.space1);
        let url = escape_xml(request.url.to_source().as_str());
        self.fmt_span("url", &url);
        self.fmt_lt(&request.line_terminator0);
        request.headers.iter().for_each(|h| self.fmt_kv(h));
        request.sections.iter().for_each(|s| self.fmt_section(s));
        if let Some(body) = &request.body {
            self.fmt_body(body);
        }
        self.fmt_span_close();
    }

    fn fmt_response(&mut self, response: &Response) {
        self.fmt_span_open("response");
        self.fmt_lts(&response.line_terminators);
        self.fmt_space(&response.space0);
        self.fmt_version(&response.version);
        self.fmt_space(&response.space1);
        self.fmt_status(&response.status);
        self.fmt_lt(&response.line_terminator0);
        response.headers.iter().for_each(|h| self.fmt_kv(h));
        response.sections.iter().for_each(|s| self.fmt_section(s));
        if let Some(body) = &response.body {
            self.fmt_body(body);
        }
        self.fmt_span_close();
    }

    fn fmt_method(&mut self, method: &Method) {
        self.fmt_span("method", &method.to_string());
    }

    fn fmt_version(&mut self, version: &Version) {
        self.fmt_span("version", &version.value.to_string());
    }

    fn fmt_status(&mut self, status: &Status) {
        self.fmt_number(status.value.to_string());
    }

    fn fmt_section(&mut self, section: &Section) {
        self.fmt_lts(&section.line_terminators);
        self.fmt_space(&section.space0);
        let name = format!("[{}]", section.identifier());
        self.fmt_span("section-header", &name);
        self.fmt_lt(&section.line_terminator0);
        self.fmt_section_value(&section.value);
    }

    fn fmt_section_value(&mut self, section_value: &SectionValue) {
        match section_value {
            SectionValue::Asserts(items) => items.iter().for_each(|item| self.fmt_assert(item)),
            SectionValue::QueryParams(items, _) => items.iter().for_each(|item| self.fmt_kv(item)),
            SectionValue::BasicAuth(item) => {
                if let Some(kv) = item {
                    self.fmt_kv(kv);
                }
            }
            SectionValue::FormParams(items, _) => items.iter().for_each(|item| self.fmt_kv(item)),
            SectionValue::MultipartFormData(items, _) => {
                items.iter().for_each(|item| self.fmt_multipart_param(item));
            }
            SectionValue::Cookies(items) => items.iter().for_each(|item| self.fmt_cookie(item)),
            SectionValue::Captures(items) => items.iter().for_each(|item| self.fmt_capture(item)),
            SectionValue::Options(items) => {
                items.iter().for_each(|item| self.fmt_entry_option(item));
            }
        }
    }

    fn fmt_kv(&mut self, kv: &KeyValue) {
        self.fmt_lts(&kv.line_terminators);
        self.fmt_space(&kv.space0);
        self.fmt_template(&kv.key);
        self.fmt_space(&kv.space1);
        self.buffer.push(':');
        self.fmt_space(&kv.space2);
        self.fmt_template(&kv.value);
        self.fmt_lt(&kv.line_terminator0);
    }

    fn fmt_entry_option(&mut self, option: &EntryOption) {
        self.fmt_lts(&option.line_terminators);
        self.fmt_space(&option.space0);
        self.fmt_string(option.kind.identifier());
        self.fmt_space(&option.space1);
        self.buffer.push(':');
        self.fmt_space(&option.space2);
        match &option.kind {
            OptionKind::AwsSigV4(value) => self.fmt_template(value),
            OptionKind::CaCertificate(filename) => self.fmt_filename(filename),
            OptionKind::ClientCert(filename) => self.fmt_filename(filename),
            OptionKind::ClientKey(filename) => self.fmt_filename(filename),
            OptionKind::Compressed(value) => self.fmt_bool_option(value),
            OptionKind::ConnectTo(value) => self.fmt_template(value),
            OptionKind::ConnectTimeout(value) => self.fmt_duration_option(value),
            OptionKind::Delay(value) => self.fmt_duration_option(value),
            OptionKind::FollowLocation(value) => self.fmt_bool_option(value),
            OptionKind::FollowLocationTrusted(value) => self.fmt_bool_option(value),
            OptionKind::Header(value) => self.fmt_template(value),
            OptionKind::Http10(value) => self.fmt_bool_option(value),
            OptionKind::Http11(value) => self.fmt_bool_option(value),
            OptionKind::Http2(value) => self.fmt_bool_option(value),
            OptionKind::Http3(value) => self.fmt_bool_option(value),
            OptionKind::Insecure(value) => self.fmt_bool_option(value),
            OptionKind::IpV4(value) => self.fmt_bool_option(value),
            OptionKind::IpV6(value) => self.fmt_bool_option(value),
            OptionKind::LimitRate(value) => self.fmt_natural_option(value),
            OptionKind::MaxRedirect(value) => self.fmt_count_option(value),
            OptionKind::MaxTime(value) => self.fmt_duration_option(value),
            OptionKind::NetRc(value) => self.fmt_bool_option(value),
            OptionKind::NetRcFile(filename) => self.fmt_filename(filename),
            OptionKind::NetRcOptional(value) => self.fmt_bool_option(value),
            OptionKind::Output(filename) => self.fmt_filename(filename),
            OptionKind::PathAsIs(value) => self.fmt_bool_option(value),
            OptionKind::PinnedPublicKey(value) => self.fmt_template(value),
            OptionKind::Proxy(value) => self.fmt_template(value),
            OptionKind::Repeat(value) => self.fmt_count_option(value),
            OptionKind::Resolve(value) => self.fmt_template(value),
            OptionKind::Retry(value) => self.fmt_count_option(value),
            OptionKind::RetryInterval(value) => self.fmt_duration_option(value),
            OptionKind::Skip(value) => self.fmt_bool_option(value),
            OptionKind::UnixSocket(value) => self.fmt_filename(value),
            OptionKind::User(value) => self.fmt_template(value),
            OptionKind::Variable(value) => self.fmt_variable_definition(value),
            OptionKind::Verbose(value) => self.fmt_bool_option(value),
            OptionKind::VeryVerbose(value) => self.fmt_bool_option(value),
        };
        self.fmt_lt(&option.line_terminator0);
    }

    fn fmt_count_option(&mut self, count_option: &CountOption) {
        match count_option {
            CountOption::Literal(repeat) => self.fmt_count(*repeat),
            CountOption::Placeholder(placeholder) => self.fmt_placeholder(placeholder),
        }
    }

    fn fmt_count(&mut self, count: Count) {
        match count {
            Count::Finite(n) => self.fmt_number(n),
            Count::Infinite => self.fmt_number(-1),
        };
    }

    fn fmt_variable_definition(&mut self, option: &VariableDefinition) {
        self.buffer.push_str(option.name.as_str());
        self.fmt_space(&option.space0);
        self.buffer.push('=');
        self.fmt_space(&option.space1);
        self.fmt_variable_value(&option.value);
    }

    fn fmt_variable_value(&mut self, option: &VariableValue) {
        match option {
            VariableValue::Null => self.fmt_span("null", "null"),
            VariableValue::Bool(v) => self.fmt_bool(*v),
            VariableValue::Number(v) => self.fmt_number(v.to_source()),
            VariableValue::String(t) => self.fmt_template(t),
        }
    }

    fn fmt_multipart_param(&mut self, param: &MultipartParam) {
        match param {
            MultipartParam::Param(param) => self.fmt_kv(param),
            MultipartParam::FilenameParam(param) => self.fmt_file_param(param),
        };
    }

    fn fmt_file_param(&mut self, param: &FilenameParam) {
        self.fmt_lts(&param.line_terminators);
        self.fmt_space(&param.space0);
        self.fmt_template(&param.key);
        self.fmt_space(&param.space1);
        self.buffer.push(':');
        self.fmt_space(&param.space2);
        self.fmt_file_value(&param.value);
        self.fmt_lt(&param.line_terminator0);
    }

    fn fmt_file_value(&mut self, file_value: &FilenameValue) {
        self.buffer.push_str("file,");
        self.fmt_space(&file_value.space0);
        self.fmt_filename(&file_value.filename);
        self.fmt_space(&file_value.space1);
        self.buffer.push(';');
        self.fmt_space(&file_value.space2);
        if let Some(content_type) = &file_value.content_type {
            self.fmt_template(content_type);
        }
    }

    fn fmt_filename(&mut self, filename: &Template) {
        self.fmt_span_open("filename");
        let s = filename.to_string().replace(' ', "\\ ");
        self.buffer.push_str(s.as_str());
        self.fmt_span_close();
    }

    fn fmt_cookie(&mut self, cookie: &Cookie) {
        self.fmt_lts(&cookie.line_terminators);
        self.fmt_space(&cookie.space0);
        self.fmt_template(&cookie.name);
        self.fmt_space(&cookie.space1);
        self.buffer.push(':');
        self.fmt_space(&cookie.space2);
        self.fmt_template(&cookie.value);
        self.fmt_lt(&cookie.line_terminator0);
    }

    fn fmt_capture(&mut self, capture: &Capture) {
        self.fmt_lts(&capture.line_terminators);
        self.fmt_space(&capture.space0);
        self.fmt_template(&capture.name);
        self.fmt_space(&capture.space1);
        self.buffer.push(':');
        self.fmt_space(&capture.space2);
        self.fmt_query(&capture.query);
        for (space, filter) in capture.filters.iter() {
            self.fmt_space(space);
            self.fmt_filter(filter);
        }
        self.fmt_space(&capture.space3);
        if capture.redact {
            self.fmt_string("redact");
        }
        self.fmt_lt(&capture.line_terminator0);
    }

    fn fmt_query(&mut self, query: &Query) {
        self.fmt_query_value(&query.value);
    }

    fn fmt_query_value(&mut self, query_value: &QueryValue) {
        let query_type = query_value.identifier();
        self.fmt_span("query-type", query_type);
        match query_value {
            QueryValue::Header { space0, name } => {
                self.fmt_space(space0);
                self.fmt_template(name);
            }
            QueryValue::Cookie { space0, expr } => {
                self.fmt_space(space0);
                self.fmt_cookie_path(expr);
            }
            QueryValue::Xpath { space0, expr } => {
                self.fmt_space(space0);
                self.fmt_template(expr);
            }
            QueryValue::Jsonpath { space0, expr } => {
                self.fmt_space(space0);
                self.fmt_template(expr);
            }
            QueryValue::Regex { space0, value } => {
                self.fmt_space(space0);
                self.fmt_regex_value(value);
            }
            QueryValue::Variable { space0, name } => {
                self.fmt_space(space0);
                self.fmt_template(name);
            }
            QueryValue::Certificate {
                space0,
                attribute_name: field,
            } => {
                self.fmt_space(space0);
                self.fmt_certificate_attribute_name(field);
            }
            QueryValue::Status
            | QueryValue::Version
            | QueryValue::Url
            | QueryValue::Body
            | QueryValue::Duration
            | QueryValue::Bytes
            | QueryValue::Sha256
            | QueryValue::Md5
            | QueryValue::Ip
            | QueryValue::Redirects => {}
        }
    }

    fn fmt_regex_value(&mut self, regex_value: &RegexValue) {
        match regex_value {
            RegexValue::Template(template) => self.fmt_template(template),
            RegexValue::Regex(regex) => self.fmt_regex(regex),
        }
    }

    fn fmt_cookie_path(&mut self, cookie_path: &CookiePath) {
        self.fmt_span_open("string");
        self.buffer.push('"');
        self.buffer.push_str(cookie_path.name.to_source().as_str());
        if let Some(attribute) = &cookie_path.attribute {
            self.buffer.push('[');
            self.fmt_cookie_attribute(attribute);
            self.buffer.push(']');
        }
        self.buffer.push('"');
        self.fmt_span_close();
    }

    fn fmt_cookie_attribute(&mut self, cookie_attribute: &CookieAttribute) {
        self.fmt_space(&cookie_attribute.space0);
        self.buffer.push_str(cookie_attribute.name.value().as_str());
        self.fmt_space(&cookie_attribute.space1);
    }

    fn fmt_certificate_attribute_name(&mut self, name: &CertificateAttributeName) {
        self.fmt_span_open("string");
        self.buffer.push('"');
        self.buffer.push_str(name.identifier());
        self.buffer.push('"');
        self.fmt_span_close();
    }

    fn fmt_assert(&mut self, assert: &Assert) {
        self.fmt_lts(&assert.line_terminators);
        self.fmt_space(&assert.space0);
        self.fmt_query(&assert.query);
        for (space, filter) in assert.filters.iter() {
            self.fmt_space(space);
            self.fmt_filter(filter);
        }
        self.fmt_space(&assert.space1);
        self.fmt_predicate(&assert.predicate);
        self.fmt_lt(&assert.line_terminator0);
    }

    fn fmt_predicate(&mut self, predicate: &Predicate) {
        if predicate.not {
            self.fmt_span("not", "not");
            self.fmt_space(&predicate.space0);
        }
        self.fmt_predicate_func(&predicate.predicate_func);
    }

    fn fmt_predicate_func(&mut self, predicate_func: &PredicateFunc) {
        self.fmt_predicate_func_value(&predicate_func.value);
    }

    fn fmt_predicate_func_value(&mut self, value: &PredicateFuncValue) {
        self.fmt_span_open("predicate-type");
        self.buffer.push_str(&encode_html(value.identifier()));
        self.fmt_span_close();

        match value {
            PredicateFuncValue::Equal { space0, value, .. } => {
                self.fmt_space(space0);
                self.fmt_predicate_value(value);
            }
            PredicateFuncValue::NotEqual { space0, value, .. } => {
                self.fmt_space(space0);
                self.fmt_predicate_value(value);
            }
            PredicateFuncValue::GreaterThan { space0, value, .. } => {
                self.fmt_space(space0);
                self.fmt_predicate_value(value);
            }
            PredicateFuncValue::GreaterThanOrEqual { space0, value, .. } => {
                self.fmt_space(space0);
                self.fmt_predicate_value(value);
            }
            PredicateFuncValue::LessThan { space0, value, .. } => {
                self.fmt_space(space0);
                self.fmt_predicate_value(value);
            }
            PredicateFuncValue::LessThanOrEqual { space0, value, .. } => {
                self.fmt_space(space0);
                self.fmt_predicate_value(value);
            }
            PredicateFuncValue::StartWith { space0, value } => {
                self.fmt_space(space0);
                self.fmt_predicate_value(value);
            }
            PredicateFuncValue::EndWith { space0, value } => {
                self.fmt_space(space0);
                self.fmt_predicate_value(value);
            }
            PredicateFuncValue::Contain { space0, value } => {
                self.fmt_space(space0);
                self.fmt_predicate_value(value);
            }
            PredicateFuncValue::Include { space0, value } => {
                self.fmt_space(space0);
                self.fmt_predicate_value(value);
            }
            PredicateFuncValue::Match { space0, value } => {
                self.fmt_space(space0);
                self.fmt_predicate_value(value);
            }
            PredicateFuncValue::IsInteger => {}
            PredicateFuncValue::IsFloat => {}
            PredicateFuncValue::IsBoolean => {}
            PredicateFuncValue::IsString => {}
            PredicateFuncValue::IsCollection => {}
            PredicateFuncValue::IsDate => {}
            PredicateFuncValue::IsIsoDate => {}
            PredicateFuncValue::Exist => {}
            PredicateFuncValue::IsEmpty => {}
            PredicateFuncValue::IsNumber => {}
            PredicateFuncValue::IsIpv4 => {}
            PredicateFuncValue::IsIpv6 => {}
        }
    }

    fn fmt_predicate_value(&mut self, predicate_value: &PredicateValue) {
        match predicate_value {
            PredicateValue::String(value) => self.fmt_template(value),
            PredicateValue::MultilineString(value) => self.fmt_multiline_string(value),
            PredicateValue::Number(value) => self.fmt_number(value.to_source()),
            PredicateValue::Bool(value) => self.fmt_bool(*value),
            PredicateValue::File(value) => self.fmt_file(value),
            PredicateValue::Hex(value) => self.fmt_hex(value),
            PredicateValue::Base64(value) => self.fmt_base64(value),
            PredicateValue::Placeholder(value) => self.fmt_placeholder(value),
            PredicateValue::Null => self.fmt_span("null", "null"),
            PredicateValue::Regex(value) => self.fmt_regex(value),
        };
    }

    fn fmt_multiline_string(&mut self, multiline_string: &MultilineString) {
        let body = multiline_string.to_source();
        let body = escape_xml(body.as_str());
        self.fmt_span("multiline", &body);
    }

    fn fmt_body(&mut self, body: &Body) {
        self.fmt_lts(&body.line_terminators);
        self.fmt_space(&body.space0);
        self.fmt_bytes(&body.value);
        let lt = &body.line_terminator0;
        self.fmt_space(&lt.space0);
        if let Some(v) = &lt.comment {
            self.fmt_comment(v);
        }
        self.buffer.push_str(lt.newline.as_str());
    }

    fn fmt_bytes(&mut self, bytes: &Bytes) {
        match bytes {
            Bytes::Base64(value) => {
                self.fmt_base64(value);
            }
            Bytes::File(value) => {
                self.fmt_file(value);
            }
            Bytes::Hex(value) => {
                self.fmt_hex(value);
            }
            Bytes::OnelineString(value) => {
                self.fmt_template(value);
            }
            Bytes::Json(value) => self.fmt_json_value(value),
            Bytes::MultilineString(value) => self.fmt_multiline_string(value),
            Bytes::Xml(value) => self.fmt_xml(value),
        }
    }

    fn fmt_string(&mut self, value: &str) {
        self.fmt_span("string", value);
    }

    fn fmt_bool(&mut self, value: bool) {
        self.fmt_span("boolean", &value.to_string());
    }

    fn fmt_bool_option(&mut self, value: &BooleanOption) {
        match value {
            BooleanOption::Literal(value) => self.fmt_span("boolean", &value.to_string()),
            BooleanOption::Placeholder(value) => self.fmt_placeholder(value),
        }
    }

    fn fmt_natural_option(&mut self, value: &NaturalOption) {
        match value {
            NaturalOption::Literal(value) => {
                self.fmt_span("number", &value.to_source().to_string());
            }
            NaturalOption::Placeholder(value) => self.fmt_placeholder(value),
        }
    }

    fn fmt_duration_option(&mut self, value: &DurationOption) {
        match value {
            DurationOption::Literal(literal) => {
                self.fmt_span("number", &literal.value.to_source().to_string());
                if let Some(unit) = literal.unit {
                    self.fmt_span("unit", &unit.to_string());
                }
            }
            DurationOption::Placeholder(value) => self.fmt_placeholder(value),
        }
    }

    fn fmt_number<T: Sized + Display>(&mut self, value: T) {
        self.fmt_span("number", &value.to_string());
    }

    fn fmt_xml(&mut self, value: &str) {
        let value = escape_xml(value);
        self.fmt_span("xml", &value);
    }

    fn fmt_json_value(&mut self, json_value: &JsonValue) {
        let json = json_value.to_source();
        let json = escape_xml(json.as_str());
        self.fmt_span("json", &json);
    }

    fn fmt_space(&mut self, space: &Whitespace) {
        let Whitespace { value, .. } = space;
        if !value.is_empty() {
            self.buffer.push_str(value);
        };
    }

    fn fmt_lt(&mut self, lt: &LineTerminator) {
        self.fmt_space(&lt.space0);
        if let Some(v) = &lt.comment {
            self.fmt_comment(v);
        }
        self.buffer.push_str(lt.newline.as_str());
    }

    fn fmt_comment(&mut self, comment: &Comment) {
        let comment = format!("#{}", escape_xml(&comment.value));
        self.fmt_span("comment", &comment);
    }

    fn fmt_file(&mut self, file: &File) {
        self.buffer.push_str("file,");
        self.fmt_space(&file.space0);
        self.fmt_filename(&file.filename);
        self.fmt_space(&file.space1);
        self.buffer.push(';');
    }

    fn fmt_base64(&mut self, base64: &Base64) {
        self.buffer.push_str("base64,");
        self.fmt_space(&base64.space0);
        self.fmt_span("base64", &base64.source.to_string());
        self.fmt_space(&base64.space1);
        self.buffer.push(';');
    }

    fn fmt_hex(&mut self, hex: &Hex) {
        self.buffer.push_str("hex,");
        self.fmt_space(&hex.space0);
        self.fmt_span("hex", &hex.source.to_string());
        self.fmt_space(&hex.space1);
        self.buffer.push(';');
    }

    fn fmt_regex(&mut self, regex: &Regex) {
        self.fmt_span("regex", regex.to_source().as_str());
    }

    fn fmt_template(&mut self, template: &Template) {
        let s = template.to_source();
        self.fmt_string(&escape_xml(s.as_str()));
    }

    fn fmt_placeholder(&mut self, placeholder: &Placeholder) {
        let placeholder = placeholder.to_source();
        self.fmt_span("expr", placeholder.as_str());
    }

    fn fmt_filter(&mut self, filter: &Filter) {
        self.fmt_filter_value(&filter.value);
    }

    fn fmt_filter_value(&mut self, filter_value: &FilterValue) {
        self.fmt_span("filter-type", filter_value.identifier());
        match filter_value {
            FilterValue::Decode { space0, encoding } => {
                self.fmt_space(space0);
                self.fmt_template(encoding);
            }
            FilterValue::Format { space0, fmt } => {
                self.fmt_space(space0);
                self.fmt_template(fmt);
            }
            FilterValue::JsonPath { space0, expr } => {
                self.fmt_space(space0);
                self.fmt_template(expr);
            }
            FilterValue::Nth { space0, n: value } => {
                self.fmt_space(space0);
                self.fmt_number(value.to_source());
            }
            FilterValue::Regex { space0, value } => {
                self.fmt_space(space0);
                self.fmt_regex_value(value);
            }
            FilterValue::Replace {
                space0,
                old_value,
                space1,
                new_value,
            } => {
                self.fmt_space(space0);
                self.fmt_template(old_value);
                self.fmt_space(space1);
                self.fmt_template(new_value);
            }
            FilterValue::ReplaceRegex {
                space0,
                pattern,
                space1,
                new_value,
            } => {
                self.fmt_space(space0);
                self.fmt_regex_value(pattern);
                self.fmt_space(space1);
                self.fmt_template(new_value);
            }
            FilterValue::Split { space0, sep } => {
                self.fmt_space(space0);
                self.fmt_template(sep);
            }
            FilterValue::ToDate { space0, fmt } => {
                self.fmt_space(space0);
                self.fmt_template(fmt);
            }
            FilterValue::UrlQueryParam { space0, param } => {
                self.fmt_space(space0);
                self.fmt_template(param);
            }
            FilterValue::XPath { space0, expr } => {
                self.fmt_space(space0);
                self.fmt_template(expr);
            }
            FilterValue::Base64Decode
            | FilterValue::Base64Encode
            | FilterValue::Base64UrlSafeDecode
            | FilterValue::Base64UrlSafeEncode
            | FilterValue::Count
            | FilterValue::DaysAfterNow
            | FilterValue::DaysBeforeNow
            | FilterValue::First
            | FilterValue::HtmlEscape
            | FilterValue::HtmlUnescape
            | FilterValue::Last
            | FilterValue::Location
            | FilterValue::ToFloat
            | FilterValue::ToHex
            | FilterValue::ToInt
            | FilterValue::ToString
            | FilterValue::UrlDecode
            | FilterValue::UrlEncode => {}
        };
    }

    fn fmt_lts(&mut self, line_terminators: &[LineTerminator]) {
        for lt in line_terminators {
            self.fmt_space(&lt.space0);
            if let Some(v) = &lt.comment {
                self.fmt_comment(v);
            }
            if !lt.newline.value.is_empty() {
                self.buffer.push_str(lt.newline.as_str());
            }
        }
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn encode_html(s: &str) -> String {
    s.replace('>', "&gt;").replace('<', "&lt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{JsonObjectElement, MultilineStringKind, SourceInfo, TemplateElement};
    use crate::reader::Pos;
    use crate::typing::ToSource;

    #[test]
    fn test_multiline_string() {
        // ```
        // line1
        // line2
        // ```
        let kind = MultilineStringKind::Text(Template {
            delimiter: None,
            elements: vec![TemplateElement::String {
                value: "line1\nline2\n".to_string(),
                source: "line1\nline2\n".to_source(),
            }],
            source_info: SourceInfo::new(Pos::new(0, 0), Pos::new(0, 0)),
        });
        let attributes = vec![];
        let multiline_string = MultilineString {
            attributes,
            space: Whitespace {
                value: String::new(),
                source_info: SourceInfo {
                    start: Pos { line: 1, column: 4 },
                    end: Pos { line: 1, column: 4 },
                },
            },
            newline: Whitespace {
                value: "\n".to_string(),
                source_info: SourceInfo {
                    start: Pos { line: 1, column: 4 },
                    end: Pos { line: 2, column: 1 },
                },
            },
            kind,
        };
        let mut fmt = HtmlFormatter::new();
        fmt.fmt_multiline_string(&multiline_string);
        assert_eq!(
            fmt.buffer,
            "<span class=\"multiline\">```\nline1\nline2\n```</span>"
        );
    }

    #[test]
    fn test_json() {
        let mut fmt = HtmlFormatter::new();
        let value = JsonValue::Object {
            space0: String::new(),
            elements: vec![JsonObjectElement {
                space0: "\n   ".to_string(),
                name: Template::new(
                    Some('"'),
                    vec![TemplateElement::String {
                        value: "id".to_string(),
                        source: "id".to_source(),
                    }],
                    SourceInfo::new(Pos::new(0, 0), Pos::new(0, 0)),
                ),
                space1: String::new(),
                space2: " ".to_string(),
                value: JsonValue::Number("1".to_string()),
                space3: "\n".to_string(),
            }],
        };
        fmt.fmt_json_value(&value);
        assert_eq!(fmt.buffer, "<span class=\"json\">{\n   \"id\": 1\n}</span>");
    }

    #[test]
    fn test_json_encoded_newline() {
        let mut fmt = HtmlFormatter::new();
        let value = JsonValue::String(Template::new(
            Some('"'),
            vec![TemplateElement::String {
                value: "\n".to_string(),
                source: "\\n".to_source(),
            }],
            SourceInfo::new(Pos::new(0, 0), Pos::new(0, 0)),
        ));
        fmt.fmt_json_value(&value);
        assert_eq!(fmt.buffer, "<span class=\"json\">\"\\n\"</span>");
    }

    #[test]
    fn test_xml() {
        let mut fmt = HtmlFormatter::new();
        let value = "<?xml version=\"1.0\"?>\n<drink>café</drink>";
        fmt.fmt_xml(value);
        assert_eq!(
            fmt.buffer,
            "<span class=\"xml\">&lt;?xml version=\"1.0\"?&gt;\n&lt;drink&gt;café&lt;/drink&gt;</span>"
        );
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(escape_xml("hello"), "hello");
        assert_eq!(
            escape_xml("<?xml version=\"1.0\"?>"),
            "&lt;?xml version=\"1.0\"?&gt;"
        );
    }
}
