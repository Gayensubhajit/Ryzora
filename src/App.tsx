import { ThemeProvider } from "./theme";
import React, { useState } from "react";
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
import { LockScreenDetailView } from "./views/LockScreenDetailView";
import { AboutModal } from "./components/AboutModal";
import { FirstRunBanner } from "./components/FirstRunBanner";
import { Toast } from "./components/Toast";

const MainLayout: React.FC = () => {
  const { activeCategory, selectedPackage } = useApp();
  const [aboutOpen, setAboutOpen] = useState(false);

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
    <div className="flex h-screen w-screen overflow-hidden bg-[var(--rz-bg)] text-[var(--rz-text)] antialiased font-sans opacity-100">
      {/* Fixed Navigation Sidebar */}
      <Sidebar onOpenAbout={() => setAboutOpen(true)} />

      {/* Main Content Pane */}
      <div className="flex-1 flex flex-col h-screen overflow-hidden min-w-0">
        {/* Top Header */}
        <TopBar onOpenAbout={() => setAboutOpen(true)} />

        {/* Viewport */}
        <main className="flex-1 overflow-y-auto min-w-0 relative">
          <div>
            {/* Dismissible First-Run Onboarding Banner */}
            <div className="px-4 sm:px-6 pt-2">
              <FirstRunBanner />
            </div>

            {/* Active Content View */}
            {activeCategory === "discover" ? (
              renderActiveView()
            ) : (
              <div className="px-4 sm:px-6 py-4 sm:py-6">
                {renderActiveView()}
              </div>
            )}
          </div>
        </main>
      </div>

      {/* Dedicated Lock Screen Product View vs Generic Package Detail Modal */}
      {selectedPackage && (selectedPackage.category === "lockscreens" || selectedPackage.package_type === "lockscreen") ? (
        <LockScreenDetailView />
      ) : (
        <PackageDetailModal />
      )}

      {/* Compact Native About Modal */}
      <AboutModal isOpen={aboutOpen} onClose={() => setAboutOpen(false)} />

      {/* Notification Toast */}
      <Toast />
    </div>
  );
};

export default function App() {
  return (
    <ThemeProvider>
      <AppProvider>
        <MainLayout />
      </AppProvider>
    </ThemeProvider>
  );
}
