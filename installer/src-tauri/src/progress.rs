use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Running,
    Done,
    Failed,
}

#[derive(Serialize, Clone, Debug)]
pub struct StepPlan {
    pub id: String,
    pub label: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Event {
    Plan { steps: Vec<StepPlan> },
    Step { id: String, status: Status, message: Option<String> },
    Download { done: u64, total: Option<u64> },
}

#[derive(Serialize, Clone, Debug)]
pub struct StepResult {
    pub id: String,
    pub label: String,
    pub status: Status,
    pub message: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub version: String,
    pub steps: Vec<StepResult>,
    pub failed: usize,
}

pub struct Steps<'a> {
    report: &'a mut dyn FnMut(Event),
    plan: Vec<StepPlan>,
    results: Vec<StepResult>,
}

impl<'a> Steps<'a> {
    pub fn new(report: &'a mut dyn FnMut(Event), plan: Vec<StepPlan>) -> Steps<'a> {
        report(Event::Plan { steps: plan.clone() });
        Steps {
            report,
            plan,
            results: Vec::new(),
        }
    }

    fn label(&self, id: &str) -> String {
        self.plan
            .iter()
            .find(|step| step.id == id)
            .map(|step| step.label.clone())
            .unwrap_or_else(|| id.to_string())
    }

    pub fn start(&mut self, id: &str) {
        (self.report)(Event::Step {
            id: id.to_string(),
            status: Status::Running,
            message: None,
        });
    }

    pub fn download(&mut self, done: u64, total: Option<u64>) {
        (self.report)(Event::Download { done, total });
    }

    pub fn finish(&mut self, id: &str, outcome: Result<Option<String>, String>) -> bool {
        let (status, message) = match outcome {
            Ok(note) => (Status::Done, note),
            Err(error) => (Status::Failed, Some(error)),
        };
        (self.report)(Event::Step {
            id: id.to_string(),
            status,
            message: message.clone(),
        });
        self.results.push(StepResult {
            id: id.to_string(),
            label: self.label(id),
            status,
            message,
        });
        status == Status::Done
    }

    pub fn into_summary(self, version: String) -> Summary {
        let failed = self.results.iter().filter(|r| r.status == Status::Failed).count();
        Summary {
            version,
            steps: self.results,
            failed,
        }
    }
}
