// Navbar.tsx
import { ReactNode } from "react";

export interface Tab {
  id: string;
  label: string;
  icon?: string;
}

interface NavbarProps {
  tabs: Tab[];
  activeTab: string;
  onTabChange: (tabId: string) => void;
  children: ReactNode; // Contenido del tab activo (ya filtrado afuera)
}

export function Navbar({
  tabs,
  activeTab,
  onTabChange,
  children,
}: NavbarProps) {
  return (
    <div className="flex flex-col h-full">
      <nav className="flex gap-1 border-b border-zinc-800 bg-zinc-950 px-4 pt-3">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => onTabChange(tab.id)}
            className={`
              relative px-4 py-2.5 text-sm font-medium rounded-t-lg transition-colors
              ${
                activeTab === tab.id
                  ? "text-white bg-zinc-900 border-t border-x border-zinc-700 -mb-px"
                  : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/50"
              }
            `}
          >
            <span className="flex items-center gap-2">
              {tab.icon && <span>{tab.icon}</span>}
              {tab.label}
            </span>
          </button>
        ))}
      </nav>

      <div className="flex-1 overflow-auto bg-zinc-950 p-4">{children}</div>
    </div>
  );
}
