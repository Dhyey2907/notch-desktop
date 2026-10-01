import {
  File,
  FileArchive,
  FileAudio,
  FileCode,
  FileImage,
  FileSpreadsheet,
  FileText,
  FileVideo,
  Folder,
  type LucideIcon,
} from "lucide-react";

const EXT_ICONS: [LucideIcon, string[]][] = [
  [FileImage, ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "heic", "ico"]],
  [FileVideo, ["mp4", "mov", "mkv", "avi", "webm"]],
  [FileAudio, ["mp3", "wav", "flac", "m4a", "ogg"]],
  [FileArchive, ["zip", "rar", "7z", "tar", "gz"]],
  [FileSpreadsheet, ["xlsx", "xls", "csv"]],
  [FileCode, ["js", "ts", "tsx", "rs", "py", "json", "html", "css", "md"]],
  [FileText, ["pdf", "doc", "docx", "txt", "rtf", "pptx"]],
];

export function FileIcon({ ext, isDir }: { ext?: string; isDir?: boolean }) {
  const Icon = isDir ? Folder : (EXT_ICONS.find(([, exts]) => ext && exts.includes(ext))?.[0] ?? File);
  return <Icon size={14} />;
}
