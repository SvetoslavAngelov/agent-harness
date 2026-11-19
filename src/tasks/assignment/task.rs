use rand::{RngExt, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde::Serialize;

pub struct User {
    pub user_id: i64,
    pub name: String,
}

#[derive(Serialize)]
pub struct Job {
    pub job_id: i64,
    pub name: String,
}

#[derive(Serialize)]
pub struct Assignment {
    pub user_id: i64,
    pub user_name: String,
    pub job_id: i64,
    pub job_name: String,
}

pub struct Intent {
    pub user_id: Option<i64>,
    pub job_id: i64,
    pub others_job_id: Option<i64>,
}

impl Intent {
    pub fn expected_job_id(&self, user_id: i64, initial_job_id: i64) -> i64 {
        match self.user_id {
            None => self.job_id,
            Some(target) if target == user_id => self.job_id,
            Some(_) => self.others_job_id.unwrap_or(initial_job_id),
        }
    }
}

pub struct Task {
    pub instruction: String,
    pub task_type: &'static str,
    pub intent: Intent,
    pub initial: Vec<Assignment>,
}

pub fn generate(mut users: Vec<User>, jobs: &[Job], seed: u64) -> Task {
    let mut rng = StdRng::seed_from_u64(seed);
    users.shuffle(&mut rng);
    let initial: Vec<_> = users
        .into_iter()
        .take(3)
        .map(|user| {
            let job = &jobs[rng.random_range(0..jobs.len())];
            Assignment {
                user_id: user.user_id,
                user_name: user.name,
                job_id: job.job_id,
                job_name: job.name.clone(),
            }
        })
        .collect();

    let user = &initial[rng.random_range(0..initial.len())];
    let job = &jobs[rng.random_range(0..jobs.len())];
    let (task_type, instruction, intent) = match rng.random_range(0..3) {
        0 => (
            "everyone",
            format!("Assign everyone to {}.", initial[0].job_name),
            Intent {
                user_id: None,
                job_id: initial[0].job_id,
                others_job_id: None,
            },
        ),
        1 => (
            "one_user",
            format!(
                "Assign user {} to {}. Leave everyone else's assignment unchanged.",
                user.user_id, job.name
            ),
            Intent {
                user_id: Some(user.user_id),
                job_id: job.job_id,
                others_job_id: None,
            },
        ),
        _ => {
            let other_jobs: Vec<_> = jobs
                .iter()
                .filter(|other| other.job_id != job.job_id)
                .collect();
            let other_job = other_jobs[rng.random_range(0..other_jobs.len())];
            (
                "one_and_others",
                format!(
                    "Assign user {} to {} and everyone else to {}.",
                    user.user_id, job.name, other_job.name
                ),
                Intent {
                    user_id: Some(user.user_id),
                    job_id: job.job_id,
                    others_job_id: Some(other_job.job_id),
                },
            )
        }
    };

    Task {
        instruction,
        task_type,
        intent,
        initial,
    }
}
