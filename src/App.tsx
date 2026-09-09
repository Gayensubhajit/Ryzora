import React from "react";
import { AppProvider, useApp } from "./context/AppContext";
import { Sidebar } from "./components/Sidebar";
import { TopBar } from "./components/TopBar";
import { HubView } from "./views/HubView";
import { UpdatesView } from "./views/UpdatesView";
import { RepositoryView } from "./views/RepositoryView";
import { CreatorProfileView } from "./views/CreatorProfileView";
import { DiscoverView } from "./views/DiscoverView";
import { CategoryView } from "./views/CategoryView";
import { InstalledView } from "./views/InstalledView";
import { BackupsView } from "./views/BackupsView";
import { SystemView } from "./views/SystemView";
import { AuthorView } from "./views/AuthorView";
import { SettingsView } from "./views/SettingsView";
import { NotificationsView } from "./views/NotificationsView";
import { CollectionsView } from "./views/CollectionsView";
import { IntegrityView } from "./views/IntegrityView";
import { PackageDetailModal } from "./components/PackageDetailModal";
import { Toast } from "./components/Toast";

const MainLayout: React.FC = () => {
  const { activeCategory } = useApp();

  const renderActiveView = () => {
    switch (activeCategory) {
      case "hub":
        return <HubView />;
      case "discover":
        return <DiscoverView />;
      case "updates":
        return <UpdatesView />;
      case "repositories":
        return <RepositoryView />;
      case "creators":
        return <CreatorProfileView />;
      case "installed":
        return <InstalledView />;
      case "backups":
        return <BackupsView />;
      case "system":
        return <SystemView />;
      case "author":
        return <AuthorView />;
      case "settings":
        return <SettingsView />;
      case "notifications":
        return <NotificationsView />;
      case "collections":
        return <CollectionsView />;
      case "integrity":
        return <IntegrityView />;
      default:
        return <CategoryView />;
    }
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-[var(--bg-canvas)] text-[var(--text-primary)] antialiased font-sans">
      {/* Fixed Navigation Sidebar */}
      <Sidebar />

      {/* Main Content Pane */}
      <div className="flex-1 flex flex-col h-screen overflow-hidden">
        {/* Top Header */}
        <TopBar />

        {/* Viewport */}
        <main className="flex-1 overflow-y-auto px-6 py-6">
          <div className="max-w-6xl mx-auto">
            {renderActiveView()}
          </div>
        </main>
      </div>

      {/* Package Detail Modal Dialog */}
      <PackageDetailModal />

      {/* Notification Toast */}
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
