#include "mpv_video_item.h"

#include <QtCore/QByteArray>
#include <QtCore/QMetaObject>
#include <QtGui/QOpenGLContext>
#include <QtGui/QOpenGLFunctions>
#include <QtOpenGL/QOpenGLFramebufferObject>
#include <QtQuick/QQuickOpenGLUtils>
#include <QtQuick/QQuickWindow>
#include <QtQml/qqml.h>

#include <mpv/client.h>
#include <mpv/render_gl.h>

namespace {

static void *get_proc_address(void *, const char *name)
{
  auto *context = QOpenGLContext::currentContext();
  if (!context) {
    return nullptr;
  }

  return reinterpret_cast<void *>(context->getProcAddress(name));
}

static void on_mpv_update(void *ctx)
{
  auto *item = static_cast<MpvVideoItem *>(ctx);
  if (!item) {
    return;
  }

  QMetaObject::invokeMethod(item, "update", Qt::QueuedConnection);
}

class MpvVideoRenderer final : public QQuickFramebufferObject::Renderer
{
public:
  ~MpvVideoRenderer() override { destroyRenderContext(); }

  void synchronize(QQuickFramebufferObject *item) override
  {
    auto *mpvItem = static_cast<MpvVideoItem *>(item);
    m_item = mpvItem;

    bool ok = false;
    auto handleValue = mpvItem->mpvHandleToken().toULongLong(&ok);
    auto handle = ok ? handleValue : 0;

    if (m_mpvHandle == handle) {
      return;
    }

    m_mpvHandle = handle;
    destroyRenderContext();
  }

  QOpenGLFramebufferObject *createFramebufferObject(const QSize &size) override
  {
    QOpenGLFramebufferObjectFormat format;
    format.setAttachment(QOpenGLFramebufferObject::CombinedDepthStencil);
    return new QOpenGLFramebufferObject(size, format);
  }

  void render() override
  {
    auto *context = QOpenGLContext::currentContext();
    if (!context) {
      return;
    }

    auto *functions = context->functions();
    functions->glClearColor(0.04f, 0.05f, 0.07f, 1.0f);
    functions->glClear(GL_COLOR_BUFFER_BIT);

    ensureRenderContext();
    if (!m_renderContext) {
      return;
    }

    auto *fbo = framebufferObject();
    if (!fbo) {
      return;
    }

    mpv_opengl_fbo mpvFbo {
      static_cast<int>(fbo->handle()),
      fbo->width(),
      fbo->height(),
      0,
    };
    int flipY = 1;
    mpv_render_param params[] = {
      {MPV_RENDER_PARAM_OPENGL_FBO, &mpvFbo},
      {MPV_RENDER_PARAM_FLIP_Y, &flipY},
      {MPV_RENDER_PARAM_INVALID, nullptr},
    };

    mpv_render_context_render(m_renderContext, params);
    QQuickOpenGLUtils::resetOpenGLState();
  }

private:
  void ensureRenderContext()
  {
    if (m_renderContext || m_mpvHandle == 0 || !m_item) {
      return;
    }

    auto *handle = reinterpret_cast<mpv_handle *>(static_cast<quintptr>(m_mpvHandle));
    mpv_opengl_init_params glInitParams {
      get_proc_address,
      nullptr,
    };

    mpv_render_param params[] = {
      {MPV_RENDER_PARAM_API_TYPE, const_cast<char *>(MPV_RENDER_API_TYPE_OPENGL)},
      {MPV_RENDER_PARAM_OPENGL_INIT_PARAMS, &glInitParams},
      {MPV_RENDER_PARAM_INVALID, nullptr},
    };

    if (mpv_render_context_create(&m_renderContext, handle, params) < 0) {
      m_renderContext = nullptr;
      return;
    }

    mpv_render_context_set_update_callback(m_renderContext, on_mpv_update, m_item);
  }

  void destroyRenderContext()
  {
    if (!m_renderContext) {
      return;
    }

    mpv_render_context_set_update_callback(m_renderContext, nullptr, nullptr);
    mpv_render_context_free(m_renderContext);
    m_renderContext = nullptr;
  }

  MpvVideoItem *m_item = nullptr;
  qulonglong m_mpvHandle = 0;
  mpv_render_context *m_renderContext = nullptr;
};

} // namespace

MpvVideoItem::MpvVideoItem(QQuickItem *parent)
  : QQuickFramebufferObject(parent)
{
  setTextureFollowsItemSize(true);
  setMirrorVertically(true);
}

MpvVideoItem::~MpvVideoItem() = default;

QQuickFramebufferObject::Renderer *MpvVideoItem::createRenderer() const
{
  return new MpvVideoRenderer();
}

QString MpvVideoItem::mpvHandleToken() const
{
  return m_mpvHandleToken;
}

void MpvVideoItem::setMpvHandleToken(const QString &handleToken)
{
  if (m_mpvHandleToken == handleToken) {
    return;
  }

  m_mpvHandleToken = handleToken;
  bool ok = false;
  m_mpvHandle = handleToken.toULongLong(&ok);
  if (!ok) {
    m_mpvHandle = 0;
  }
  emit mpvHandleTokenChanged();
  update();
}

extern "C" void els_register_mpv_video_item()
{
  qmlRegisterType<MpvVideoItem>("com.yfhao.els.mpv", 1, 0, "MpvVideoItem");
}
