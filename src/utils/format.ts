/** 把字节数格式化成人类可读的容量，null 显示为占位符。 */
export function formatBytes(value: number | null): string {
  if (value === null) return "—";
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(0)} KiB`;
  if (value < 1024 * 1024 * 1024) {
    return `${(value / 1024 / 1024).toFixed(1)} MiB`;
  }
  return `${(value / 1024 / 1024 / 1024).toFixed(2)} GiB`;
}
