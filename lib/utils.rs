pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use screeps::game::{cpu, time};
use crate::log;
use lazy_static::lazy_static;

lazy_static! {
    // pub static ref JPS_PROFILER: Arc<Profiler> = Arc::new(Profiler::new());
    pub static ref PROFILER: Profiler = Profiler::new();
}

pub struct Profiler {
    calls: RwLock<HashMap<String, (usize, f64)>>, // (count, total_time)
    start_times: RwLock<HashMap<String, f64>>,
    start_tick: RwLock<u32>,
    total_ticks: RwLock<u32>,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            calls: RwLock::new(HashMap::new()),
            start_times: RwLock::new(HashMap::new()),
            start_tick: RwLock::new(time()),
            total_ticks: RwLock::new(1),
        }
    }

    pub fn start_call(&self, name: &str) {
        let current_tick = time();
        let mut total_ticks = self.total_ticks.write().unwrap();
        if current_tick != *self.start_tick.write().unwrap() {
            *total_ticks += 1;
            *self.start_tick.write().unwrap() = current_tick;
        }
        self.start_times.write().unwrap().insert(name.to_string(), cpu::get_used());
    }

    pub fn end_call(&self, name: &str) {
        if let Some(start_time) = self.start_times.write().unwrap().remove(name) {
            let duration = cpu::get_used() - start_time;
            let mut calls = self.calls.write().unwrap();
            let entry = calls.entry(name.to_string()).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += duration;
        }
    }

    pub fn get_results(&self) -> HashMap<String, ProfileStats> {
        let mut results = HashMap::new();
        let total_ticks = *self.total_ticks.read().unwrap() as f64;
        
        for (name, (count, total_time)) in self.calls.read().unwrap().iter() {
            let avg_time = total_time / *count as f64;
            let calls_per_tick = *count as f64 / total_ticks;
            let cpu_per_tick = total_time / total_ticks;
            
            results.insert(name.clone(), ProfileStats {
                count: *count,
                total_time: *total_time,
                avg_time,
                calls_per_tick,
                cpu_per_tick,
            });
        }
        
        results
    }

    pub fn print_results(&self) {
        let mut stats: Vec<_> = self.get_results().into_iter().collect();
        stats.sort_by(|a, b| b.1.total_time.partial_cmp(&a.1.total_time).unwrap());

        // Calculate column widths
        let mut name_width = 9; // "Operation"
        let mut count_width = 5; // "Count"
        let mut total_width = 9; // "Total CPU"
        let mut avg_width = 8; // "Avg CPU"
        let mut cpu_tick_width = 8; // "CPU/tick"
        let mut calls_tick_width = 10; // "Calls/tick"

        for (name, stats) in &stats {
            name_width = name_width.max(name.len());
            count_width = count_width.max(format!("{}", stats.count).len());
            total_width = total_width.max(format!("{:.2}", stats.total_time).len());
            avg_width = avg_width.max(format!("{:.4}", stats.avg_time).len());
            cpu_tick_width = cpu_tick_width.max(format!("{:.4}", stats.cpu_per_tick).len());
            calls_tick_width = calls_tick_width.max(format!("{:.2}", stats.calls_per_tick).len());
        }

        let mut table = format!("\nProfiling Results (over {} ticks):\n", *self.total_ticks.read().unwrap());
        
        // Header
        table.push_str(&format!(
            "| {:<width_name$} | {:>width_count$} | {:>width_total$} | {:>width_avg$} | {:>width_cpu$} | {:>width_calls$} |\n",
            "Operation", "Count", "Total CPU", "Avg CPU", "CPU/tick", "Calls/tick",
            width_name = name_width,
            width_count = count_width,
            width_total = total_width,
            width_avg = avg_width,
            width_cpu = cpu_tick_width,
            width_calls = calls_tick_width
        ));

        // Separator
        table.push_str(&format!(
            "|{:-<width_name$}-|{:-<width_count$}-|{:-<width_total$}-|{:-<width_avg$}-|{:-<width_cpu$}-|{:-<width_calls$}-|\n",
            "", "", "", "", "", "",
            width_name = name_width + 2,
            width_count = count_width + 2,
            width_total = total_width + 2,
            width_avg = avg_width + 2,
            width_cpu = cpu_tick_width + 2,
            width_calls = calls_tick_width + 2
        ));
        
        // Data rows
        for (name, stats) in stats {
            table.push_str(&format!(
                "| {:<width_name$} | {:>width_count$} | {:>width_total$.2} | {:>width_avg$.4} | {:>width_cpu$.4} | {:>width_calls$.2} |\n",
                name, stats.count, stats.total_time, stats.avg_time, stats.cpu_per_tick, stats.calls_per_tick,
                width_name = name_width,
                width_count = count_width,
                width_total = total_width,
                width_avg = avg_width,
                width_cpu = cpu_tick_width,
                width_calls = calls_tick_width
            ));
        }

        unsafe {
            log(&table);
        }
    }

    pub fn reset(&self) {
        self.calls.write().unwrap().clear();
        self.start_times.write().unwrap().clear();
        *self.start_tick.write().unwrap() = time();
        *self.total_ticks.write().unwrap() = 1;
    }
}

#[derive(Debug)]
pub struct ProfileStats {
    pub count: usize,
    pub total_time: f64,
    pub avg_time: f64,
    pub calls_per_tick: f64,
    pub cpu_per_tick: f64,
}

