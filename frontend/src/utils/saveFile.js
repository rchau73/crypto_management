// Hands a Blob to the browser as a file download. A temporary <a download>
// is the only cross-browser way to name the file without a new dependency.
export function saveFile(blob, filename) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}
