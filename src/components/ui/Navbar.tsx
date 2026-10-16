import { ReactNode } from "react";
import { ThemeToggle } from "./ThemeToggle";

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
    <div className="flex flex-col h-full w-full overflow-hidden select-none bg-canvas">
      {/* Contenedor del Navbar con flex-wrap */}
      <div className="w-full border-b border-border bg-canvas p-3 sm:px-4 sm:pt-3">
        <div className="flex items-center gap-2 w-full">
          <nav role="tablist" aria-label="Secciones" className="flex flex-wrap gap-2 sm:gap-1.5 w-full flex-1">
            {tabs.map((tab) => {
              const isActive = activeTab === tab.id;
              return (
                <button
                  key={tab.id}
                  role="tab"
                  aria-selected={isActive}
                  onClick={() => onTabChange(tab.id)}
                  className={`
                    flex-1 sm:flex-none
                    flex items-center justify-center gap-2 px-3.5 py-2 text-sm font-medium transition-colors
                    rounded-t-lg rounded-b-none
                    ${
                      isActive
                        ? "text-text-primary bg-surface border-b-2 border-focus"
                        : "text-text-secondary hover:text-text-primary bg-surface/40 hover:bg-surface/60 border-b-2 border-transparent"
                    }
                  `}
                >
                  {tab.icon && (
                    <span className="text-base" aria-hidden="true">
                      {tab.icon}
                    </span>
                  )}
                  <span className="truncate">{tab.label}</span>
                </button>
              );
            })}
          </nav>
          <ThemeToggle />
        </div>
      </div>

      {/* Contenido principal */}
      <div className="flex-1 w-full overflow-auto bg-canvas p-4">
        {children}
      </div>
    </div>
  );
}
