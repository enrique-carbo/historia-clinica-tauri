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
  children: ReactNode;
}

export function Navbar({
  tabs,
  activeTab,
  onTabChange,
  children,
}: NavbarProps) {
  return (
    <div className="flex flex-col h-full w-full overflow-hidden select-none bg-zinc-950">
      {/* Contenedor del Navbar con flex-wrap */}
      <div className="w-full border-b border-zinc-800 bg-zinc-950 p-3 sm:px-4 sm:pt-3">
        <nav className="flex flex-wrap gap-2 sm:gap-1.5 w-full">
          {tabs.map((tab) => {
            const isActive = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => onTabChange(tab.id)}
                className={`
                  flex-1 sm:flex-none
                  flex items-center justify-center gap-2 px-3.5 py-2 text-sm font-medium transition-all
                  rounded-t-lg rounded-b-none
                  ${
                    isActive
                      ? "text-white bg-zinc-900 border-b-2 border-indigo-500"
                      : "text-zinc-400 hover:text-zinc-200 bg-zinc-900/40 hover:bg-zinc-900/60 border-b-2 border-transparent"
                  }
                `}
              >
                {tab.icon && <span className="text-base">{tab.icon}</span>}
                <span className="truncate">{tab.label}</span>
              </button>
            );
          })}
        </nav>
      </div>

      {/* Contenido principal */}
      <div className="flex-1 w-full overflow-auto bg-zinc-950 p-4">
        {children}
      </div>
    </div>
  );
}
