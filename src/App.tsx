import React from "react";
import { AppProvider, useApp } from "./context/AppContext";
import { Sidebar } from "./components/Sidebar";
import { TopBar } from "./components/TopBar";
import { DiscoverView } from "./views/DiscoverView";
import { CategoryView } from "./views/CategoryView";
import { InstalledView } from "./views/InstalledView";
import { BackupsView } from "./views/BackupsView";
import { SystemView } from "./views/SystemView";
import { PackageDetailModal } from "./components/PackageDetailModal";
import { Toast } from "./components/Toast";

const MainLayout: React.FC = () => {
  const { activeCategory } = useApp();

  const renderActiveView = () => {
    switch (activeCategory) {
      case "discover":
        return <DiscoverView />;
      case "installed":
        return <InstalledView />;
      case "backups":
        return <BackupsView />;
      case "system":
        return <SystemView />;
      default:
        return <CategoryView />;
    }
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-[#07090e] text-slate-100 antialiased font-sans">
      {/* Fixed Left Navigation Sidebar */}
      <Sidebar />

      {/* Main Content Pane */}
      <div className="flex-1 flex flex-col h-screen overflow-hidden">
        {/* Top Header with Search and Desktop Filter */}
        <TopBar />

        {/* Dynamic Viewport Container */}
        <main className="flex-1 overflow-y-auto px-6 py-8">
          <div className="max-w-7xl mx-auto">
            {renderActiveView()}
          </div>
        </main>
      </div>

      {/* Package Detail Modal & Safety Inspector */}
      <PackageDetailModal />

      {/* Toast Notification */}
      <Toast />
    </div>
  );
};

export default function App() {
  return (
    <AppProvider>
      <MainLayout />
    </AppProvider>
  );
}
