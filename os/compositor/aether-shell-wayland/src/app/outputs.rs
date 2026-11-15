//! Output registration, hotplug removal and pointer-to-output lookup.

use aether_geometry::Point as AetherPoint;
use aether_shell::{OutputId, OutputInfo};
use smithay::output::Output;
use smithay::reexports::wayland_server::backend::GlobalId;

use super::App;

pub(crate) struct OutputEntry {
    pub(crate) output: Output,
    global: GlobalId,
}

impl App {
    /// Output currently under the pointer, falling back to the primary.
    pub fn output_under_pointer(&self) -> OutputId {
        let point = AetherPoint::new(
            self.pointer_pos.x.round() as i32,
            self.pointer_pos.y.round() as i32,
        );
        self.shell
            .output_at(point)
            .or_else(|| self.shell.primary_output().map(|output| output.id))
            .unwrap_or(1)
    }

    /// Register a Wayland output and extend the shell's desktop. The output
    /// must already have its preferred/current mode set by the backend.
    pub fn add_output(&mut self, info: OutputInfo, output: Output) -> GlobalId {
        let global = output.create_global::<App>(&self.display_handle);
        let mut outputs: Vec<OutputInfo> = self
            .shell
            .outputs()
            .iter()
            .filter(|candidate| candidate.id != info.id)
            .cloned()
            .collect();
        outputs.push(info.clone());
        outputs.sort_by_key(|candidate| candidate.id);
        if !outputs.iter().any(|candidate| candidate.primary) {
            if let Some(first) = outputs.first_mut() {
                first.primary = true;
            }
        }
        self.shell.set_outputs(outputs);
        for surface in self
            .windows
            .values()
            .map(|toplevel| toplevel.wl_surface().clone())
            .collect::<Vec<_>>()
        {
            output.enter(&surface);
        }
        println!(
            "aether-shell: output {} {}x{}@{}Hz at {}+{} ({} total)",
            info.name,
            info.rect.width,
            info.rect.height,
            info.refresh_hz,
            info.rect.x,
            info.rect.y,
            self.shell.outputs().len()
        );
        self.outputs.insert(
            info.id,
            OutputEntry {
                output,
                global: global.clone(),
            },
        );
        self.configure_all();
        self.mark_full_damage();
        global
    }

    /// Remove an output (hotplug). Windows that lived on it fall back to the
    /// primary output in the shell policy.
    pub fn remove_output(&mut self, id: OutputId) -> Option<Output> {
        let entry = self.outputs.remove(&id)?;
        for surface in self
            .windows
            .values()
            .map(|toplevel| toplevel.wl_surface().clone())
            .collect::<Vec<_>>()
        {
            entry.output.leave(&surface);
        }
        self.display_handle.remove_global::<App>(entry.global);
        let remaining: Vec<OutputInfo> = self
            .shell
            .outputs()
            .iter()
            .filter(|candidate| candidate.id != id)
            .cloned()
            .collect();
        if !remaining.is_empty() {
            self.shell.set_outputs(remaining);
        }
        self.sync_focus();
        self.configure_all();
        println!(
            "aether-shell: output {id} removed ({} remaining)",
            self.shell.outputs().len()
        );
        Some(entry.output)
    }

    pub fn output(&self, id: OutputId) -> Option<&Output> {
        self.outputs.get(&id).map(|entry| &entry.output)
    }

    pub fn output_ids(&self) -> Vec<OutputId> {
        self.shell
            .outputs()
            .iter()
            .map(|output| output.id)
            .collect()
    }
}
