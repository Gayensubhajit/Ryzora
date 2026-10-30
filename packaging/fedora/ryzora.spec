Name:           ryzora
Version:        0.1.0
Release:        1%{?dist}
Summary:        Universal Linux Desktop Customization Platform

License:        MIT
URL:            https://github.com/Gayensubhajit/Ryzora
Source0:        https://github.com/Gayensubhajit/Ryzora/archive/refs/tags/v%{version}.tar.gz#/%{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.75
BuildRequires:  cargo
BuildRequires:  cmake
BuildRequires:  ninja-build
BuildRequires:  gcc
BuildRequires:  gcc-c++
BuildRequires:  pkgconfig
BuildRequires:  openssl-devel
BuildRequires:  qt6-qtbase-devel
BuildRequires:  qt6-qtdeclarative-devel
BuildRequires:  desktop-file-utils
BuildRequires:  libappstream-glib

Requires:       qt6-qtbase
Requires:       qt6-qtdeclarative
Requires:       openssl
Requires:       hicolor-icon-theme
Suggests:       hyprland
Suggests:       hyprlock
Suggests:       waybar
Suggests:       fastfetch
Suggests:       kitty

%description
Ryzora is a universal Linux desktop customization manager and software center.
It allows users to safely discover, preview, install, configure, and roll back
desktop rices, Waybar configs, lock screens, and themes without risking system breakage.

%prep
%autosetup -n Ryzora-%{version}

%build
cmake -B qt/build -S qt -DCMAKE_BUILD_TYPE=Release
cmake --build qt/build --parallel

%install
# Install main binary
install -Dm755 qt/build/ryzora %{buildroot}%{_bindir}/ryzora

# Desktop entries
install -Dm644 dist-assets/io.ryzora.Ryzora.desktop %{buildroot}%{_datadir}/applications/io.ryzora.Ryzora.desktop
ln -sf io.ryzora.Ryzora.desktop %{buildroot}%{_datadir}/applications/ryzora.desktop

# AppStream metadata
install -Dm644 dist-assets/io.ryzora.Ryzora.metainfo.xml %{buildroot}%{_metainfodir}/io.ryzora.Ryzora.metainfo.xml

# Icons
install -Dm644 dist-assets/icons/hicolor/scalable/apps/ryzora.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/ryzora.svg
for size in 16 32 48 64 128 256 512; do
    if [ -f dist-assets/icons/hicolor/${size}x${size}/apps/ryzora.png ]; then
        install -Dm644 dist-assets/icons/hicolor/${size}x${size}/apps/ryzora.png \
            %{buildroot}%{_datadir}/icons/hicolor/${size}x${size}/apps/ryzora.png
    fi
done

# Privileged helpers and Polkit security policies
install -Dm755 crates/ryzora-core/resources/ryzora-sddm-helper %{buildroot}%{_prefix}/lib/ryzora/ryzora-sddm-helper
install -Dm755 crates/ryzora-core/resources/ryzora-package-helper %{buildroot}%{_prefix}/lib/ryzora/ryzora-package-helper
install -Dm644 crates/ryzora-core/resources/io.ryzora.sddm.policy %{buildroot}%{_datadir}/polkit-1/actions/io.ryzora.sddm.policy
install -Dm644 crates/ryzora-core/resources/io.ryzora.package.policy %{buildroot}%{_datadir}/polkit-1/actions/io.ryzora.package.policy
install -Dm644 crates/ryzora-core/resources/io.ryzora.sddm.rules %{buildroot}%{_datadir}/polkit-1/rules.d/io.ryzora.sddm.rules
install -Dm644 crates/ryzora-core/resources/io.ryzora.package.rules %{buildroot}%{_datadir}/polkit-1/rules.d/io.ryzora.package.rules

# Licenses and docs
install -Dm644 LICENSE %{buildroot}%{_licensedir}/%{name}/LICENSE
install -Dm644 README.md %{buildroot}%{_docdir}/%{name}/README.md

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/io.ryzora.Ryzora.desktop || true

%files
%license %{_licensedir}/%{name}/LICENSE
%doc %{_docdir}/%{name}/README.md
%{_bindir}/ryzora
%{_datadir}/applications/io.ryzora.Ryzora.desktop
%{_datadir}/applications/ryzora.desktop
%{_metainfodir}/io.ryzora.Ryzora.metainfo.xml
%{_datadir}/icons/hicolor/*/apps/ryzora.*
%{_prefix}/lib/ryzora/
%{_datadir}/polkit-1/actions/io.ryzora.*.policy
%{_datadir}/polkit-1/rules.d/io.ryzora.*.rules

%changelog
* Wed Sep 30 2026 Subhajit Gayen <subhajitgayen43@gmail.com> - 0.1.0-1
- Initial release for Fedora / RPM ecosystems
