use human_sort::compare;

#[derive(Debug, PartialEq)]
pub enum ParseKind {
    Unknown,
    Req,
    Cal,
    Sem,
}

#[derive(Debug, PartialEq)]
pub enum ParsePart {
    Start,
    ReqPrefix,
    MajorYear,
    MinorMonth,
    PatchDay,
    PreId,
    BuildSuffix,
}

pub fn parse_multi(input: impl AsRef<str>) -> Vec<String> {
    let input = input.as_ref();
    let mut results = vec![];

    if input.contains("||") {
        let mut parts = input.split("||").collect::<Vec<_>>();

        // Try and sort from highest to lowest range
        parts.sort_by(|a, d| compare(d, a));

        for part in parts {
            results.extend(parse_multi(part));
        }
    } else if input.contains(',') {
        results.push(
            input
                .split(",")
                .map(|part| do_parse(part, true).0)
                .collect::<Vec<_>>()
                .join(","),
        );
    } else {
        results.push(do_parse(input, false).0);
    }

    results
}

pub fn parse(input: impl AsRef<str>) -> (String, ParseKind) {
    do_parse(input, false)
}

pub fn do_parse(input: impl AsRef<str>, within_and: bool) -> (String, ParseKind) {
    let input = input.as_ref();
    let mut kind = ParseKind::Unknown;
    let mut in_part = ParsePart::Start;

    // Track each part
    let mut req_op = String::new();
    let mut major_year = String::new();
    let mut minor_month = String::new();
    let mut patch_day = String::new();
    let mut pre_id = String::new();
    let mut build_id = String::new();

    // let debug = |ch: char| {
    //     dbg!(
    //         input,
    //         ch,
    //         &kind,
    //         &in_part,
    //         &req_op,
    //         &major_year,
    //         &minor_month,
    //         &patch_day,
    //         &pre_id,
    //         &build_id
    //     );
    // };

    for ch in input.chars() {
        match ch {
            // Requirement operator
            '=' | '~' | '^' | '>' | '<' => {
                if in_part != ParsePart::Start && in_part != ParsePart::ReqPrefix {
                    panic!("Requirement operator found in an invalid position");
                }

                in_part = ParsePart::ReqPrefix;
                req_op.push(ch);
            }
            // Wildcard operator
            '*' => {
                // Ignore entirely
            }
            // Version part
            '0'..='9' => {
                let part_str = match in_part {
                    ParsePart::Start | ParsePart::ReqPrefix | ParsePart::MajorYear => {
                        in_part = ParsePart::MajorYear;
                        &mut major_year
                    }
                    ParsePart::MinorMonth => &mut minor_month,
                    ParsePart::PatchDay => &mut patch_day,
                    ParsePart::PreId => &mut pre_id,
                    ParsePart::BuildSuffix => &mut build_id,
                };

                if part_str.is_empty()
                    && ch == '0'
                    && matches!(
                        in_part,
                        ParsePart::MajorYear | ParsePart::MinorMonth | ParsePart::PatchDay
                    )
                {
                    // Trim leading zero's
                } else {
                    part_str.push(ch);
                }
            }
            // Suffix part
            'a'..='z' | 'A'..='Z' => match in_part {
                ParsePart::PreId => {
                    pre_id.push(ch);
                }
                ParsePart::BuildSuffix => {
                    build_id.push(ch);
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
                if kind == ParseKind::Unknown {
                    if ch == '-' {
                        kind = ParseKind::Cal;
                    } else {
                        kind = ParseKind::Sem;
                    }
                }

                // Continue to the next part
                if ch == '-' {
                    if kind == ParseKind::Sem {
                        match in_part {
                            ParsePart::MajorYear | ParsePart::MinorMonth | ParsePart::PatchDay => {
                                in_part = ParsePart::PreId;
                            }
                            ParsePart::PreId => {
                                pre_id.push('-');
                            }
                            ParsePart::BuildSuffix => {
                                build_id.push('-');
                            }
                            _ => unreachable!(),
                        };
                    } else if kind == ParseKind::Cal {
                        match in_part {
                            ParsePart::MajorYear => {
                                in_part = ParsePart::MinorMonth;
                            }
                            ParsePart::MinorMonth => {
                                in_part = ParsePart::PatchDay;
                            }
                            ParsePart::PatchDay | ParsePart::BuildSuffix => {
                                in_part = ParsePart::PreId;
                            }
                            ParsePart::PreId => {
                                pre_id.push('-');
                            }
                            _ => unreachable!(),
                        };
                    }
                } else if ch == '.' {
                    if kind == ParseKind::Sem {
                        match in_part {
                            ParsePart::MajorYear => {
                                in_part = ParsePart::MinorMonth;
                            }
                            ParsePart::MinorMonth => {
                                in_part = ParsePart::PatchDay;
                            }
                            ParsePart::PatchDay => {
                                in_part = ParsePart::PreId;
                            }
                            ParsePart::PreId => {
                                pre_id.push('.');
                            }
                            ParsePart::BuildSuffix => {
                                build_id.push('.');
                            }
                            _ => unreachable!(),
                        };
                    } else if kind == ParseKind::Cal {
                        match in_part {
                            ParsePart::MajorYear | ParsePart::MinorMonth | ParsePart::PatchDay => {
                                in_part = ParsePart::BuildSuffix;
                            }
                            ParsePart::PreId => {
                                pre_id.push('.');
                            }
                            ParsePart::BuildSuffix => {
                                build_id.push('.');
                            }
                            _ => unreachable!(),
                        };
                    }
                }
            }
            // Build separator
            '_' | '+' => {
                if ch == '+' {
                    if kind == ParseKind::Sem {
                        in_part = ParsePart::BuildSuffix;
                    } else {
                        unreachable!();
                    }
                } else {
                    if kind == ParseKind::Cal {
                        in_part = ParsePart::BuildSuffix;
                    } else {
                        unreachable!();
                    }
                }
            }
            // Whitespace
            ' ' => {
                // Just skip
            }
            _ => {
                dbg!(
                    input,
                    ch,
                    &kind,
                    &in_part,
                    &req_op,
                    &major_year,
                    &minor_month,
                    &patch_day,
                    &pre_id,
                    &build_id
                );
                panic!("Unknown character `{}` in version string!", ch)
            }
        }
    }

    // Rebuild the parts
    let mut output = String::new();
    let was_calver = kind == ParseKind::Cal;

    if req_op.is_empty() {
        if minor_month.is_empty() || patch_day.is_empty() {
            kind = ParseKind::Req;

            if !within_and {
                output.push('~');
            }
        }
    } else {
        kind = ParseKind::Req;
        output.push_str(&req_op);
    }

    let separator = if kind == ParseKind::Cal { '-' } else { '.' };

    // Major/year
    if was_calver {
        if major_year.is_empty() {
            major_year.push('0');
        }

        if major_year.len() < 4 {
            let mut year: usize = major_year.parse().unwrap();
            year += 2000;

            output.push_str(&year.to_string());
        } else {
            output.push_str(&major_year);
        }
    } else if major_year.is_empty() {
        panic!("Missing major version or year!");
    } else {
        output.push_str(&major_year);
    }

    // Minor/month
    if !minor_month.is_empty() {
        output.push(separator);
        output.push_str(&minor_month);
    }

    // Patch/day
    if !patch_day.is_empty() {
        output.push(separator);
        output.push_str(&patch_day);
    }

    // Pre ID
    if !pre_id.is_empty() {
        output.push('-');
        output.push_str(&pre_id);
    }

    // Build metadata
    if !build_id.is_empty() {
        output.push('+');
        output.push_str(&build_id);
    }

    (output, kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reqs() {
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
