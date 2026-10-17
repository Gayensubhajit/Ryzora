#pragma once
// Inert SDDM greeter globals (sddm, keyboard, userModel, sessionModel) so real
// SDDM theme QML can render inside Ryzora as a visual-only preview.
// Nothing here authenticates, reads credentials, or touches the session.
#include <QAbstractListModel>
#include <QDebug>
#include <QObject>
#include <QRect>
#include <QVariantMap>

class PreviewSddm : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString hostName READ hostName CONSTANT)
    Q_PROPERTY(bool canPowerOff READ yes CONSTANT)
    Q_PROPERTY(bool canReboot READ yes CONSTANT)
    Q_PROPERTY(bool canSuspend READ yes CONSTANT)
    Q_PROPERTY(bool canHibernate READ yes CONSTANT)
    Q_PROPERTY(bool canHybridSleep READ yes CONSTANT)
public:
    explicit PreviewSddm(QObject *p = nullptr) : QObject(p) {}
    QString hostName() const { return QStringLiteral("ryzora-preview"); }
    bool yes() const { return true; }
    Q_INVOKABLE void login(const QString &, const QString &, int) { qInfo() << "[preview] login ignored"; }
    Q_INVOKABLE void powerOff() { qInfo() << "[preview] powerOff ignored"; }
    Q_INVOKABLE void reboot() { qInfo() << "[preview] reboot ignored"; }
    Q_INVOKABLE void suspend() { qInfo() << "[preview] suspend ignored"; }
    Q_INVOKABLE void hibernate() { qInfo() << "[preview] hibernate ignored"; }
    Q_INVOKABLE void hybridSleep() { qInfo() << "[preview] hybridSleep ignored"; }
signals:
    void loginFailed();
    void loginSucceeded();
    void informationMessage(const QString &message);
};

class PreviewKeyboard : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool capsLock READ caps WRITE setCaps NOTIFY changed)
    Q_PROPERTY(bool numLock READ num WRITE setNum NOTIFY changed)
    Q_PROPERTY(bool enabled READ yes CONSTANT)
    Q_PROPERTY(QVariantList layouts READ layouts CONSTANT)
    Q_PROPERTY(int currentLayout READ zero CONSTANT)
public:
    explicit PreviewKeyboard(QObject *p = nullptr) : QObject(p) {}
    bool caps() const { return m_caps; }
    bool num() const { return m_num; }
    bool yes() const { return true; }
    int zero() const { return 0; }
    QVariantList layouts() const { return {}; }
    void setCaps(bool v) { m_caps = v; emit changed(); }
    void setNum(bool v) { m_num = v; emit changed(); }
signals:
    void changed();
private:
    bool m_caps = false, m_num = false;
};

// One-row model serving both userModel and sessionModel.
class PreviewListModel : public QAbstractListModel {
    Q_OBJECT
    Q_PROPERTY(int count READ count CONSTANT)
    Q_PROPERTY(int lastIndex READ lastIndex WRITE setLastIndex NOTIFY lastIndexChanged)
    Q_PROPERTY(QString lastUser READ lastUser CONSTANT)
public:
    enum Roles { NameRole = Qt::UserRole + 1, RealNameRole, IconRole, HomeRole, FileRole, CommentRole };
    PreviewListModel(const QString &name, const QString &real, QObject *p = nullptr)
        : QAbstractListModel(p), m_name(name), m_real(real) {}
    int count() const { return 1; }
    int lastIndex() const { return m_idx; }
    void setLastIndex(int i) { m_idx = i; emit lastIndexChanged(); }
    QString lastUser() const { return m_name; }
    Q_INVOKABLE int rowCount(const QModelIndex & = QModelIndex()) const override { return 1; }
    QVariant data(const QModelIndex &idx, int role) const override {
        if (!idx.isValid() || idx.row() != 0) return {};
        switch (role) {
        case NameRole: case Qt::DisplayRole: return m_name;
        case RealNameRole: return m_real;
        case FileRole: return m_name;
        case CommentRole: return m_real;
        default: return QString();
        }
    }
    QHash<int, QByteArray> roleNames() const override {
        return {{NameRole, "name"}, {RealNameRole, "realName"}, {IconRole, "icon"},
                {HomeRole, "homeDir"}, {FileRole, "file"}, {CommentRole, "comment"}};
    }
    Q_INVOKABLE QVariantMap get(int) const {
        return {{"name", m_name}, {"realName", m_real}, {"icon", ""}, {"homeDir", ""}, {"file", m_name}, {"comment", m_real}};
    }
signals:
    void lastIndexChanged();
private:
    QString m_name, m_real;
    int m_idx = 0;
};

class PreviewScreenModel : public QObject {
    Q_OBJECT
    Q_PROPERTY(int primary READ primary CONSTANT)
public:
    explicit PreviewScreenModel(QObject *p = nullptr) : QObject(p) {}
    int primary() const { return 0; }
    Q_INVOKABLE QRect geometry(int) const { return QRect(0, 0, 1920, 1080); }
};

#include <QFile>
#include <QTextStream>
#include <QQmlPropertyMap>
#include <QRect>

// Inert SDDM theme configuration object providing stringValue, intValue, realValue, boolValue,
// and dynamic property lookup so SilentSDDM's Config.qml singleton resolves all settings.
class PreviewConfig : public QQmlPropertyMap {
    Q_OBJECT
public:
    explicit PreviewConfig(QObject *p = nullptr) : QQmlPropertyMap(this, p) {}

    void load(const QString &iniPath) {
        for (const QString &k : keys()) {
            clear(k);
        }

        QFile file(iniPath);
        if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) return;
        QTextStream in(&file);
        QString section;
        while (!in.atEnd()) {
            QString line = in.readLine().trimmed();
            if (line.isEmpty() || line.startsWith('#') || line.startsWith(';')) continue;
            if (line.startsWith('[') && line.endsWith(']')) {
                section = line.mid(1, line.length() - 2).trimmed();
                continue;
            }
            int eq = line.indexOf('=');
            if (eq < 0) continue;
            QString key = line.left(eq).trimmed();
            QString val = line.mid(eq + 1).trimmed();
            if (val.startsWith('"') && val.endsWith('"') && val.length() >= 2) {
                val = val.mid(1, val.length() - 2);
            }
            QString fullKey = section.isEmpty() ? key : (section + "/" + key);
            insert(fullKey, val);

            if (section.compare(QStringLiteral("General"), Qt::CaseInsensitive) == 0) {
                insert(key, val);
            }
        }
    }

    Q_INVOKABLE QString stringValue(const QString &key) const {
        return value(key).toString();
    }
    Q_INVOKABLE int intValue(const QString &key) const {
        return value(key).toInt();
    }
    Q_INVOKABLE qreal realValue(const QString &key) const {
        return value(key).toReal();
    }
    Q_INVOKABLE bool boolValue(const QString &key) const {
        QVariant v = value(key);
        if (v.typeId() == QMetaType::Bool) return v.toBool();
        QString s = v.toString().trimmed().toLower();
        return (s == "true" || s == "1" || s == "yes");
    }
};
