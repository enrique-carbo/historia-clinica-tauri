// Iconos de categoría de entries, en un solo lugar. Antes la misma lista
// estaba duplicada como strings en EntryForm, EntryMeta y EntryTimeline.
import {
  ChartNoAxesColumn,
  ClipboardList,
  HeartPulse,
  Pill,
  TriangleAlert,
  type LucideIcon,
} from "lucide-react";
import type { EntryCategory } from "../stores/useEntryStore";

export const CATEGORY_ICONS: Record<EntryCategory, LucideIcon> = {
  SOAP_NOTE: ClipboardList,
  ALLERGY: TriangleAlert,
  MEDICATION: Pill,
  CONDITION: HeartPulse,
};

export const ALL_ENTRIES_ICON = ChartNoAxesColumn;
