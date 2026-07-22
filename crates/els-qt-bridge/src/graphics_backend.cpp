#include "graphics_backend.h"

#include <QCoreApplication>
#include <QQuickWindow>
#include <QSGRendererInterface>
#include <Qt>

extern "C" void els_force_opengl_backend()
{
  QCoreApplication::setAttribute(Qt::AA_UseDesktopOpenGL);
  QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);
}
