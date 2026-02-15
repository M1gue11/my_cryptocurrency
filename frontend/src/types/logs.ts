// Log types matching the Rust backend LogEntry struct

export type LogLevel = "Info" | "Warning" | "Error";

export type LogCategory = "Core" | "P2P" | "RPC";

export interface LogEntry {
  timestamp: string;
  level: LogLevel;
  category: LogCategory;
  message: string;
}
