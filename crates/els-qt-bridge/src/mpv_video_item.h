#pragma once

#include <QtCore/QString>
#include <QtQuick/QQuickFramebufferObject>

class MpvVideoItem : public QQuickFramebufferObject
{
  Q_OBJECT
  Q_PROPERTY(QString mpvHandleToken READ mpvHandleToken WRITE setMpvHandleToken NOTIFY mpvHandleTokenChanged)

public:
  explicit MpvVideoItem(QQuickItem *parent = nullptr);
  ~MpvVideoItem() override;

  Renderer *createRenderer() const override;

  QString mpvHandleToken() const;
  void setMpvHandleToken(const QString &handleToken);

signals:
  void mpvHandleTokenChanged();

private:
  qulonglong m_mpvHandle = 0;
  QString m_mpvHandleToken;
};

extern "C" void els_register_mpv_video_item();
