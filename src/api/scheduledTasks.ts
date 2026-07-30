import { invoke } from "@tauri-apps/api/core";
import type {
  ScheduledTask,
  ScheduledTaskInput,
  ScheduledTaskRun,
} from "../types";

export function listScheduledTasks(): Promise<ScheduledTask[]> {
  return invoke<ScheduledTask[]>("scheduled_tasks_list");
}

export function saveScheduledTask(
  input: ScheduledTaskInput,
): Promise<ScheduledTask> {
  return invoke<ScheduledTask>("scheduled_task_save", { input });
}

export function deleteScheduledTask(id: number): Promise<void> {
  return invoke<void>("scheduled_task_delete", { id });
}

export function toggleScheduledTask(
  id: number,
  enabled: boolean,
): Promise<ScheduledTask> {
  return invoke<ScheduledTask>("scheduled_task_toggle", { id, enabled });
}

export function runScheduledTask(id: number): Promise<ScheduledTaskRun> {
  return invoke<ScheduledTaskRun>("scheduled_task_run", { id });
}

export function cancelScheduledTask(id: number): Promise<void> {
  return invoke<void>("scheduled_task_cancel", { id });
}

export function listScheduledTaskHistory(
  taskId: number | null = null,
  limit = 80,
): Promise<ScheduledTaskRun[]> {
  return invoke<ScheduledTaskRun[]>("scheduled_task_history", {
    taskId,
    limit,
  });
}
