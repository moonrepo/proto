use human_sort::compare;

#[derive(Debug, Default, PartialEq)]
pub enum ParseKind {
    #[default]
    Unknown,
    Req,
    Cal,
    Sem,
}

#[derive(Debug, Default, PartialEq)]
pub enum ParsePart {
    #[default]
    Start,
    ReqPrefix,
    MajorYear,
    MinorMonth,
    PatchDay,
    PreId,
    BuildSuffix,
}

#[derive(Debug, Default)]
pub struct UnresolvedParser {
    // States
    kind: ParseKind,
    in_part: ParsePart,
    is_and: bool,
    req_op: String,
    major_year: String,
    minor_month: String,
    patch_day: String,
    pre_id: String,
    build_id: String,

    // Final result
    results: Vec<String>,
}

impl UnresolvedParser {
    pub fn parse(mut self, input: impl AsRef<str>) -> (String, ParseKind) {
        let input = input.as_ref().trim();

        if input.is_empty() || input == "*" {
            return ("*".to_owned(), ParseKind::Req);
        }

        for ch in input.chars() {
            match ch {
                // Requirement operator
                '=' | '~' | '^' | '>' | '<' => {
                    if self.in_part != ParsePart::Start && self.in_part != ParsePart::ReqPrefix {
                        panic!("Requirement operator found in an invalid position");
                    }

                    self.in_part = ParsePart::ReqPrefix;
                    self.req_op.push(ch);
                }
                // Wildcard operator
                '*' => {
                    // Ignore entirely
                }
                // Version part
                '0'..='9' => {
                    let part_str = match self.in_part {
                        ParsePart::Start | ParsePart::ReqPrefix | ParsePart::MajorYear => {
                            self.in_part = ParsePart::MajorYear;
                            &mut self.major_year
                        }
                        ParsePart::MinorMonth => &mut self.minor_month,
                        ParsePart::PatchDay => &mut self.patch_day,
                        ParsePart::PreId => &mut self.pre_id,
                        ParsePart::BuildSuffix => &mut self.build_id,
                    };

                    part_str.push(ch);
                }
                // Suffix part
                'a'..='z' | 'A'..='Z' => match self.in_part {
                    ParsePart::PreId => {
                        self.pre_id.push(ch);
                    }
                    ParsePart::BuildSuffix => {
                        self.build_id.push(ch);
                    }
                    _ => {
                        // Remove leading v
                        if ch == 'v' || ch == 'V' {
                            continue;
                        } else {
                            unreachable!()
                        }
                    }
                },
                // Part separator
                '.' | '-' => {
                    // Determine version type based on separator
                    if self.kind == ParseKind::Unknown {
                        if ch == '-' {
                            self.kind = ParseKind::Cal;
                        } else {
                            self.kind = ParseKind::Sem;
                        }
                    }

                    // Continue to the next part
                    if ch == '-' {
                        if self.kind == ParseKind::Sem {
                            match self.in_part {
                                ParsePart::MajorYear
                                | ParsePart::MinorMonth
                                | ParsePart::PatchDay => {
                                    self.in_part = ParsePart::PreId;
                                }
                                ParsePart::PreId => {
                                    self.pre_id.push('-');
                                }
                                ParsePart::BuildSuffix => {
                                    self.build_id.push('-');
                                }
                                _ => unreachable!(),
                            };
                        } else if self.kind == ParseKind::Cal {
                            match self.in_part {
                                ParsePart::MajorYear => {
                                    self.in_part = ParsePart::MinorMonth;
                                }
                                ParsePart::MinorMonth => {
                                    self.in_part = ParsePart::PatchDay;
                                }
                                ParsePart::PatchDay | ParsePart::BuildSuffix => {
                                    self.in_part = ParsePart::PreId;
                                }
                                ParsePart::PreId => {
                                    self.pre_id.push('-');
                                }
                                _ => unreachable!(),
                            };
                        }
                    } else if ch == '.' {
                        if self.kind == ParseKind::Sem {
                            match self.in_part {
                                ParsePart::MajorYear => {
                                    self.in_part = ParsePart::MinorMonth;
                                }
                                ParsePart::MinorMonth => {
                                    self.in_part = ParsePart::PatchDay;
                                }
                                ParsePart::PatchDay => {
                                    self.in_part = ParsePart::PreId;
                                }
                                ParsePart::PreId => {
                                    self.pre_id.push('.');
                                }
                                ParsePart::BuildSuffix => {
                                    self.build_id.push('.');
                                }
                                _ => unreachable!(),
                            };
                        } else if self.kind == ParseKind::Cal {
                            match self.in_part {
                                ParsePart::MajorYear
                                | ParsePart::MinorMonth
                                | ParsePart::PatchDay => {
                                    self.in_part = ParsePart::BuildSuffix;
                                }
                                ParsePart::PreId => {
                                    self.pre_id.push('.');
                                }
                                ParsePart::BuildSuffix => {
                                    self.build_id.push('.');
                                }
                                _ => unreachable!(),
                            };
                        }
                    }
                }
                // Build separator
                '_' | '+' => {
                    if ch == '+' {
                        if self.kind == ParseKind::Sem {
                            self.in_part = ParsePart::BuildSuffix;
                        } else {
                            unreachable!();
                        }
                    } else {
                        if self.kind == ParseKind::Cal {
                            self.in_part = ParsePart::BuildSuffix;
                        } else {
                            unreachable!();
                        }
                    }
                }
                // AND separator
                ',' => {
                    self.build_result();
                    self.reset_state();
                }
                // Whitespace
                ' ' => {
                    if self.in_part == ParsePart::Start || self.in_part == ParsePart::ReqPrefix {
                        // Skip
                    } else {
                        // Possible AND sequence?
                        self.build_result();
                        self.reset_state();
                    }
                }
                _ => {
                    panic!("Unknown character `{}` in version string!", ch)
                }
            }
        }

        self.build_result();

        let result = self.get_result();
        let is_req = result.contains(',');

        (result, if is_req { ParseKind::Req } else { self.kind })
    }

    fn get_result(&self) -> String {
        self.results.join(",")
    }

    fn get_part<'p>(&self, value: &'p str) -> &'p str {
        let value = value.trim_start_matches('0');

        if value.is_empty() {
            return "0";
        }

        value
    }

    fn build_result(&mut self) {
        let mut output = String::new();
        let was_calver = self.kind == ParseKind::Cal;

        if self.req_op.is_empty() {
            if self.minor_month.is_empty() || self.patch_day.is_empty() {
                self.kind = ParseKind::Req;

                if !self.is_and {
                    output.push('~');
                }
            }
        } else {
            self.kind = ParseKind::Req;
            output.push_str(&self.req_op);
        }

        let separator = if self.kind == ParseKind::Cal {
            '-'
        } else {
            '.'
        };

        // Major/year
        if was_calver {
            let year = self.get_part(&self.major_year);

            if year.len() < 4 {
                let mut year: usize = year.parse().unwrap();
                year += 2000;

                output.push_str(&year.to_string());
            } else {
                output.push_str(year);
            }
        } else if self.major_year.is_empty() {
            panic!("Missing major version or year!");
        } else {
            output.push_str(self.get_part(&self.major_year));
        }

        // Minor/month
        if !self.minor_month.is_empty() {
            output.push(separator);
            output.push_str(self.get_part(&self.minor_month));
        }

        // Patch/day
        if !self.patch_day.is_empty() {
            output.push(separator);
            output.push_str(self.get_part(&self.patch_day));
        }

        // Pre ID
        if !self.pre_id.is_empty() {
            output.push('-');
            output.push_str(&self.pre_id);
        }

        // Build metadata
        if !self.build_id.is_empty() {
            output.push('+');
            output.push_str(&self.build_id);
        }

        self.results.push(output);
    }

    fn reset_state(&mut self) {
        self.kind = ParseKind::Unknown;
        self.in_part = ParsePart::Start;
        self.is_and = true;
        self.req_op.truncate(0);
        self.major_year.truncate(0);
        self.minor_month.truncate(0);
        self.patch_day.truncate(0);
        self.pre_id.truncate(0);
        self.build_id.truncate(0);
    }
}

pub fn parse_multi(input: impl AsRef<str>) -> Vec<String> {
    let input = input.as_ref();
    let mut results = vec![];

    if input.contains("||") {
        let mut parts = input.split("||").collect::<Vec<_>>();

        // Try and sort from highest to lowest range
        parts.sort_by(|a, d| compare(d, a));

        for part in parts {
            results.push(parse(part).0);
        }
    } else {
        results.push(parse(input).0);
    }

    results
}

pub fn parse(input: impl AsRef<str>) -> (String, ParseKind) {
    UnresolvedParser::default().parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reqs() {
        assert_eq!(parse(""), ("*".to_owned(), ParseKind::Req));
        assert_eq!(parse("*"), ("*".to_owned(), ParseKind::Req));

        // semver
        assert_eq!(parse("1"), ("~1".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.2"), ("~1.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.02"), ("~1.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("v1"), ("~1".to_owned(), ParseKind::Req));
        assert_eq!(parse("v1.2"), ("~1.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.*"), ("~1".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.*.*"), ("~1".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.2.*"), ("~1.2".to_owned(), ParseKind::Req));

        // calver
        assert_eq!(parse("2000"), ("~2000".to_owned(), ParseKind::Req));
        assert_eq!(parse("2000-2"), ("~2000.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("2000-02"), ("~2000.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("v2000"), ("~2000".to_owned(), ParseKind::Req));
        assert_eq!(parse("v2000-2"), ("~2000.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("2000-*"), ("~2000".to_owned(), ParseKind::Req));
        assert_eq!(parse("2000-*-*"), ("~2000".to_owned(), ParseKind::Req));
        assert_eq!(parse("2000-2-*"), ("~2000.2".to_owned(), ParseKind::Req));

        // calver (short years)
        assert_eq!(parse("1-2"), ("~2001.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("12-2"), ("~2012.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("123-2"), ("~2123.2".to_owned(), ParseKind::Req));

        for op in ["=", "<", "<=", ">", ">=", "~", "^"] {
            // semver
            assert_eq!(parse(format!("{op}1")), (format!("{op}1"), ParseKind::Req));
            assert_eq!(
                parse(format!("{op} 1.2")),
                (format!("{op}1.2"), ParseKind::Req)
            );
            assert_eq!(parse(format!("{op}1")), (format!("{op}1"), ParseKind::Req));
            assert_eq!(
                parse(format!("  {op}  v1.2.3  ")),
                (format!("{op}1.2.3"), ParseKind::Req)
            );

            // calver
            assert_eq!(
                parse(format!("{op}2000")),
                (format!("{op}2000"), ParseKind::Req)
            );
            assert_eq!(
                parse(format!("{op} 2000-10")),
                (format!("{op}2000.10"), ParseKind::Req)
            );
            assert_eq!(
                parse(format!("  {op}  v2000-10-03  ")),
                (format!("{op}2000.10.3"), ParseKind::Req)
            );
        }
    }

    #[test]
    fn parses_reqs_special() {
        assert_eq!(parse("1.2, 4.5"), ("1.2,4.5".to_owned(), ParseKind::Req));
        assert_eq!(
            parse(">=1.2.7 <1.3.0"),
            (">=1.2.7,<1.3.0".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse(">=1.2.0, <1.3.0"),
            (">=1.2.0,<1.3.0".to_owned(), ParseKind::Req)
        );
        assert_eq!(parse("1.2.*"), ("~1.2".to_owned(), ParseKind::Req));
        assert_eq!(
            parse(">= 1.2, < 1.5"),
            (">=1.2,<1.5".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse(">=1.2.3 <2.4.0-0"),
            (">=1.2.3,<2.4.0-0".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse(">=1.2.3, <2.4.0-0"),
            (">=1.2.3,<2.4.0-0".to_owned(), ParseKind::Req)
        );
    }

    #[test]
    fn parses_reqs_semver() {
        assert_eq!(parse("1.2.3"), ("1.2.3".to_owned(), ParseKind::Sem));
        assert_eq!(parse("01.02.03"), ("1.2.3".to_owned(), ParseKind::Sem));
        assert_eq!(parse("v1.2.3"), ("1.2.3".to_owned(), ParseKind::Sem));

        // pre
        assert_eq!(
            parse("1.2.3-alpha"),
            ("1.2.3-alpha".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("1.2.3-rc.0"),
            ("1.2.3-rc.0".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("v1.2.3-a-b-c"),
            ("1.2.3-a-b-c".to_owned(), ParseKind::Sem)
        );

        // build
        assert_eq!(
            parse("1.2.3+alpha"),
            ("1.2.3+alpha".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("1.2.3+rc.0"),
            ("1.2.3+rc.0".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("v1.2.3+a-b-c"),
            ("1.2.3+a-b-c".to_owned(), ParseKind::Sem)
        );
    }

    #[test]
    fn parses_reqs_calver() {
        assert_eq!(parse("0-2-3"), ("2000-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("00-2-3"), ("2000-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("000-2-3"), ("2000-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("1-2-3"), ("2001-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("12-2-03"), ("2012-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("123-2-31"), ("2123-2-31".to_owned(), ParseKind::Cal));
        assert_eq!(parse("2000-2-3"), ("2000-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("2000-02-03"), ("2000-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("v12-2-3"), ("2012-2-3".to_owned(), ParseKind::Cal));

        // pre
        assert_eq!(
            parse("0-2-3-rc.0"),
            ("2000-2-3-rc.0".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("v12-2-3-alpha-5"),
            ("2012-2-3-alpha-5".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("12-2-3-beta"),
            ("2012-2-3-beta".to_owned(), ParseKind::Cal)
        );

        // build
        assert_eq!(
            parse("0-2-3_123"),
            ("2000-2-3+123".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("v12-2-3_0"),
            ("2012-2-3+0".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("12-2-3.789"),
            ("2012-2-3+789".to_owned(), ParseKind::Cal)
        );
    }
}
