use crate::spec_error::SpecError;
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

impl ParsePart {
    pub fn is_prefix(&self) -> bool {
        matches!(self, Self::Start | Self::ReqPrefix)
    }

    pub fn is_suffix(&self) -> bool {
        matches!(self, Self::PreId | Self::BuildSuffix)
    }
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
    pub fn parse(mut self, input: impl AsRef<str>) -> Result<(String, ParseKind), SpecError> {
        let input = input.as_ref().trim();

        if input.is_empty() || input == "*" {
            return Ok(("*".to_owned(), ParseKind::Req));
        }

        for ch in input.chars() {
            match ch {
                // Requirement operator
                '=' | '~' | '^' | '>' | '<' => {
                    if self.in_part != ParsePart::Start && self.in_part != ParsePart::ReqPrefix {
                        return Err(SpecError::ParseInvalidReq);
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
                            return Err(SpecError::ParseUnknownChar(ch));
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
                                _ => continue,
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
                                _ => continue,
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
                                _ => continue,
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
                                _ => continue,
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
                            continue;
                        }
                    } else if self.kind == ParseKind::Cal {
                        self.in_part = ParsePart::BuildSuffix;
                    } else {
                        continue;
                    }
                }
                // AND separator
                ',' => {
                    self.is_and = true;
                    self.build_result()?;
                    self.reset_state();
                }
                // Whitespace
                ' ' => {
                    if self.in_part.is_prefix() {
                        // Skip
                    } else {
                        // Possible AND sequence?
                        self.is_and = true;
                        self.build_result()?;
                        self.reset_state();
                    }
                }
                _ => {
                    return Err(SpecError::ParseUnknownChar(ch));
                }
            }
        }

        self.build_result()?;

        let result = self.get_result();
        let is_req = result.contains(',');

        Ok((result, if is_req { ParseKind::Req } else { self.kind }))
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

    fn build_result(&mut self) -> Result<(), SpecError> {
        if self.in_part.is_prefix() {
            return Ok(());
        }

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

        let separator = if self.kind == ParseKind::Cal && !self.is_and {
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
            return Err(SpecError::ParseMissingMajorPart);
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

        Ok(())
    }

    fn reset_state(&mut self) {
        self.kind = ParseKind::Unknown;
        self.in_part = ParsePart::Start;
        self.req_op.truncate(0);
        self.major_year.truncate(0);
        self.minor_month.truncate(0);
        self.patch_day.truncate(0);
        self.pre_id.truncate(0);
        self.build_id.truncate(0);
    }
}

/// Parse the provided string as a list of version requirements,
/// as separated by `||`. Each requirement will be parsed
/// individually with [`parse`].
pub fn parse_multi(input: impl AsRef<str>) -> Result<Vec<String>, SpecError> {
    let input = input.as_ref();
    let mut results = vec![];

    if input.contains("||") {
        let mut parts = input.split("||").map(|p| p.trim()).collect::<Vec<_>>();

        // Try and sort from highest to lowest range
        parts.sort_by(|a, d| compare(d, a));

        for part in parts {
            results.push(parse(part)?.0);
        }
    } else {
        results.push(parse(input)?.0);
    }

    Ok(results)
}

/// Parse the provided string and determine the output format.
/// Since an unresolved version can be many things, such as an
/// alias, version requirement, semver, or calver, we need to
/// parse this manually to determine the correct output.
pub fn parse(input: impl AsRef<str>) -> Result<(String, ParseKind), SpecError> {
    UnresolvedParser::default().parse(input)
}
