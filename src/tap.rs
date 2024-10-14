#[cfg(windows)]
use colored::control::{set_virtual_terminal, SHOULD_COLORIZE};
use colored::Colorize;

pub trait Diagnostic {
    fn diagnostic(self);
}

impl Diagnostic for &serialport::Error {
    fn diagnostic(self) {
        println!("  {}: {:?}", "kind".green(), self.kind);
        println!("  {}: {}", "description".green(), self.description);
    }
}

impl Diagnostic for serialport::Error {
    fn diagnostic(self) {
        (&self).diagnostic();
    }
}

pub struct Tap {
    counter: usize,
    plan: usize,
}

impl Tap {
    pub fn new(plan: usize) -> Self {
        #[cfg(windows)]
        if SHOULD_COLORIZE.should_colorize() {
            set_virtual_terminal(true).ok();
        }
        println!("{}", format!("1..{plan}").bold());
        Self { counter: 0, plan }
    }

    fn ok<S>(&mut self, description: S)
    where
        S: AsRef<str>,
    {
        if self.counter < self.plan {
            self.counter += 1;
            println!(
                "{} {} - {}",
                "ok".bold(),
                self.counter.to_string().cyan(),
                description.as_ref()
            );
        }
    }

    pub fn skip<S>(&mut self, description: S)
    where
        S: AsRef<str>,
    {
        self.ok(format!("{} {}", description.as_ref(), "# SKIP".dimmed()));
    }

    fn not_ok<S, T>(&mut self, description: S, error: T)
    where
        S: AsRef<str>,
        T: Diagnostic,
    {
        if self.counter < self.plan {
            self.counter += 1;
            println!(
                "{} {} - {}",
                "not ok".red(),
                self.counter.to_string().cyan(),
                description.as_ref()
            );
            println!("  ---");
            error.diagnostic();
            println!("  ...");
        }
    }

    pub fn result<S, T, E>(&mut self, description: S, result: Result<T, E>)
    where
        S: AsRef<str>,
        E: Diagnostic,
    {
        if let Err(err) = result {
            self.not_ok(description, err);
        } else {
            self.ok(description);
        }
    }
}
