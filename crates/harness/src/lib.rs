use std::io::Write;

pub use toml_test::decoded::Decoded;
pub use toml_test::decoded::DecodedValue;
pub use toml_test::verify::Decoder;
pub use toml_test::verify::Encoder;
pub use toml_test::Error;

pub struct DecoderHarness<D> {
    decoder: D,
    matches: Option<Matches>,
}

impl<D> DecoderHarness<D>
where
    D: toml_test::verify::Decoder + Send + Sync + 'static,
{
    pub fn new(decoder: D) -> Self {
        Self {
            decoder,
            matches: None,
        }
    }

    pub fn ignore<'p>(
        &mut self,
        patterns: impl IntoIterator<Item = &'p str>,
    ) -> Result<&mut Self, toml_test::Error> {
        self.matches = Some(Matches::new(patterns.into_iter())?);
        Ok(self)
    }

    pub fn test(self) -> ! {
        let args = libtest_mimic::Arguments::from_args();
        let mut tests = Vec::new();
        tests.extend(toml_test_data::valid().map(|c| {
            libtest_mimic::Test {
                name: c.name.display().to_string(),
                kind: "".into(),
                is_ignored: self
                    .matches
                    .as_ref()
                    .map(|m| !m.matched(c.name))
                    .unwrap_or_default(),
                is_bench: false,
                data: Case::from(c),
            }
        }));
        tests.extend(toml_test_data::invalid().map(|c| {
            libtest_mimic::Test {
                name: c.name.display().to_string(),
                kind: "".into(),
                is_ignored: self
                    .matches
                    .as_ref()
                    .map(|m| !m.matched(c.name))
                    .unwrap_or_default(),
                is_bench: false,
                data: Case::from(c),
            }
        }));

        let nocapture = args.nocapture;
        libtest_mimic::run_tests(&args, tests, move |test| match test.data {
            Case::Valid(case) => {
                match self.decoder.verify_valid_case(case.fixture, case.expected) {
                    Ok(()) => libtest_mimic::Outcome::Passed,
                    Err(err) => libtest_mimic::Outcome::Failed {
                        msg: Some(err.to_string()),
                    },
                }
            }
            Case::Invalid(case) => match self.decoder.verify_invalid_case(case.fixture) {
                Ok(err) => {
                    if nocapture {
                        let _ = writeln!(std::io::stdout(), "{}", err);
                    }
                    libtest_mimic::Outcome::Passed
                }
                Err(err) => libtest_mimic::Outcome::Failed {
                    msg: Some(err.to_string()),
                },
            },
        })
        .exit()
    }
}

pub struct EncoderHarness<E, D> {
    encoder: E,
    fixture: D,
    matches: Option<Matches>,
}

impl<E, D> EncoderHarness<E, D>
where
    E: toml_test::verify::Encoder + Send + Sync + 'static,
    D: toml_test::verify::Decoder + Send + Sync + 'static,
{
    pub fn new(encoder: E, fixture: D) -> Self {
        Self {
            encoder,
            fixture,
            matches: None,
        }
    }

    pub fn ignore<'p>(
        &mut self,
        patterns: impl IntoIterator<Item = &'p str>,
    ) -> Result<&mut Self, toml_test::Error> {
        self.matches = Some(Matches::new(patterns.into_iter())?);
        Ok(self)
    }

    pub fn test(self) -> ! {
        let args = libtest_mimic::Arguments::from_args();
        let mut tests = Vec::new();
        tests.extend(toml_test_data::valid().map(|c| {
            libtest_mimic::Test {
                name: c.name.display().to_string(),
                kind: "".into(),
                is_ignored: self
                    .matches
                    .as_ref()
                    .map(|m| !m.matched(c.name))
                    .unwrap_or_default(),
                is_bench: false,
                data: Case::from(c),
            }
        }));

        libtest_mimic::run_tests(&args, tests, move |test| match test.data {
            Case::Valid(case) => {
                match self.encoder.verify_valid_case(case.expected, &self.fixture) {
                    Ok(()) => libtest_mimic::Outcome::Passed,
                    Err(err) => libtest_mimic::Outcome::Failed {
                        msg: Some(err.to_string()),
                    },
                }
            }
            Case::Invalid(_) => unreachable!("No invalid cases"),
        })
        .exit()
    }
}

enum Case {
    Valid(toml_test_data::Valid<'static>),
    Invalid(toml_test_data::Invalid<'static>),
}

impl From<toml_test_data::Valid<'static>> for Case {
    fn from(other: toml_test_data::Valid<'static>) -> Self {
        Self::Valid(other)
    }
}

impl From<toml_test_data::Invalid<'static>> for Case {
    fn from(other: toml_test_data::Invalid<'static>) -> Self {
        Self::Invalid(other)
    }
}

struct Matches {
    ignores: ignore::gitignore::Gitignore,
}

impl Matches {
    fn new<'p>(patterns: impl Iterator<Item = &'p str>) -> Result<Self, toml_test::Error> {
        let mut ignores = ignore::gitignore::GitignoreBuilder::new(".");
        for line in patterns {
            ignores
                .add_line(None, line)
                .map_err(toml_test::Error::new)?;
        }
        let ignores = ignores.build().map_err(toml_test::Error::new)?;
        Ok(Self { ignores })
    }

    fn matched(&self, path: &std::path::Path) -> bool {
        match self.ignores.matched_path_or_any_parents(path, false) {
            ignore::Match::None | ignore::Match::Whitelist(_) => true,
            ignore::Match::Ignore(_) => false,
        }
    }
}
