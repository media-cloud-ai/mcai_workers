use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, QueueableCommand};
use mcai_worker_sdk::processor::ProcessStatus;
use std::{
  collections::BTreeMap,
  io::{stdout, Write},
  sync::{
    mpsc::{self, Sender},
    Arc, Mutex
  },
};

pub struct WorkerStatuses {
  list: Arc<Mutex<BTreeMap<String, ProcessStatus>>>,
  sender: Sender<ProcessStatus>,
  max_displayed_workers: usize,
  keep_watching: bool,
}

impl WorkerStatuses {
  pub fn new() -> Self {
    let list = Arc::new(Mutex::new(BTreeMap::new()));
    let (sender, receiver) = mpsc::channel::<ProcessStatus>();

    let cloned_list = list.clone();

    std::thread::spawn(move || {
      loop {
        if let Ok(process_status) = receiver.recv() {
          let worker_id = process_status
              .worker
              .system_info
              .docker_container_id
              .clone();

          cloned_list.lock().unwrap().insert(worker_id, process_status.clone());
        }
      }
    });

    println!(
      "{:<36} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}",
      "Worker ID",
      "Used Memory",
      "Total Memory",
      "Used Swap",
      "Total Swap",
      "Nb. CPUs",
      "Activity",
      "Status"
    );

    WorkerStatuses {
      list,
      sender,
      max_displayed_workers: 0,
      keep_watching: false,
    }
  }

  pub fn set_keep_watching(&mut self) {
    self.keep_watching = true;
  }

  pub fn get_sender(&self) -> Sender<ProcessStatus> {
    self.sender.clone()
  }

  pub fn dump(&mut self) -> Result<(), String> {
    let mut stdout = stdout();

    let worker_statuses = self.list.lock().unwrap();

    let nb_workers = worker_statuses.len();
    self.max_displayed_workers = nb_workers.max(self.max_displayed_workers);
    let empty_lines = self.max_displayed_workers - nb_workers;

    for (worker_id, process_status) in worker_statuses.iter() {
      let system_info = &process_status.worker.system_info;

      let used_memory = system_info.used_memory.to_string();
      let total_memory = system_info.total_memory.to_string();
      let used_swap = system_info.used_swap.to_string();
      let total_swap = system_info.total_swap.to_string();
      let number_of_processors = system_info.number_of_processors.to_string();
      let activity = format!("{:?}", process_status.worker.activity);

      let status = process_status.job
        .as_ref()
        .map(|job_result| job_result.get_status().to_string())
        .unwrap_or("-".to_string());

      stdout
        .write(
          format!(
            "{:<36} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}\n",
            worker_id,
            used_memory,
            total_memory,
            used_swap,
            total_swap,
            number_of_processors,
            activity,
            status.as_str()
          )
          .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
    }

    for _l in 0..empty_lines {
      stdout
        .queue(Clear(ClearType::CurrentLine))
        .map_err(|e| e.to_string())?;
      stdout
        .queue(cursor::MoveToNextLine(1))
        .map_err(|e| e.to_string())?;
    }

    stdout.flush().map_err(|e| e.to_string())?;

    if self.max_displayed_workers > 0 && self.keep_watching {
      stdout
        .queue(cursor::MoveToPreviousLine(self.max_displayed_workers as u16))
        .map_err(|e| e.to_string())?;
    }
    Ok(())
  }
}