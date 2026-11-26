#include <errno.h>
#include <drm_fourcc.h>
#include <stdint.h>
#include <xf86drm.h>
#include <xf86drmMode.h>

int initium_drm_select_connector(int fd, uint32_t *connector_id)
{
    drmModeRes *resources = drmModeGetResources(fd);
    if (!resources)
        return errno ? -errno : -EIO;

    int result = -ENODEV;
    for (int i = 0; i < resources->count_connectors; ++i) {
        drmModeConnector *connector = drmModeGetConnector(fd, resources->connectors[i]);
        if (!connector)
            continue;
        if (connector->connection == DRM_MODE_CONNECTED && connector->count_modes > 0) {
            *connector_id = connector->connector_id;
            result = 0;
        }
        drmModeFreeConnector(connector);
        if (!result)
            break;
    }
    drmModeFreeResources(resources);
    return result;
}

static int find_crtc(int fd, drmModeRes *resources, uint32_t connector_id,
                     uint32_t *crtc_id)
{
    for (int i = 0; i < resources->count_connectors; ++i) {
        if (resources->connectors[i] != connector_id)
            continue;

        drmModeConnector *connector = drmModeGetConnectorCurrent(fd, resources->connectors[i]);
        if (!connector)
            continue;
        uint32_t encoder_id = connector->encoder_id;
        int connected = connector->connection == DRM_MODE_CONNECTED;
        drmModeFreeConnector(connector);
        if (!connected)
            continue;

        *crtc_id = 0;
        if (encoder_id) {
            drmModeEncoder *encoder = drmModeGetEncoder(fd, encoder_id);
            if (encoder) {
                *crtc_id = encoder->crtc_id;
                drmModeFreeEncoder(encoder);
            }
        }
        return 0;
    }
    return -ENODEV;
}

int initium_drm_scanout(int fd, uint32_t connector_id, uint32_t *framebuffer)
{
    drmModeRes *resources = drmModeGetResources(fd);
    if (!resources)
        return errno ? -errno : -EIO;

    uint32_t crtc_id;
    int result = find_crtc(fd, resources, connector_id, &crtc_id);
    drmModeFreeResources(resources);
    if (result < 0)
        return result;

    *framebuffer = 0;
    if (crtc_id) {
        drmModeCrtc *crtc = drmModeGetCrtc(fd, crtc_id);
        if (!crtc)
            return errno ? -errno : -EIO;
        *framebuffer = crtc->buffer_id;
        drmModeFreeCrtc(crtc);
    }
    return 0;
}

int initium_drm_wait_vblank(int fd, uint32_t connector_id)
{
    drmModeRes *resources = drmModeGetResources(fd);
    if (!resources)
        return errno ? -errno : -EIO;

    uint32_t crtc_id;
    int result = find_crtc(fd, resources, connector_id, &crtc_id);
    int crtc_index = -1;
    if (result == 0 && crtc_id) {
        for (int i = 0; i < resources->count_crtcs; ++i) {
            if (resources->crtcs[i] == crtc_id) {
                crtc_index = i;
                break;
            }
        }
    }
    drmModeFreeResources(resources);
    if (crtc_index < 0)
        return -ENODEV;

    drmVBlank next = {0};
    next.request.type = DRM_VBLANK_RELATIVE |
        ((uint32_t)crtc_index << DRM_VBLANK_HIGH_CRTC_SHIFT);
    next.request.sequence = 1;
    if (drmWaitVBlank(fd, &next) < 0)
        return errno ? -errno : -EIO;
    return 0;
}

struct initium_drm_mapping {
    uint64_t offset;
    uint32_t width, height, stride, length, handle;
};

int initium_drm_front_mapping(int fd, uint32_t framebuffer,
                              struct initium_drm_mapping *out)
{
    drmModeFB2 *fb = drmModeGetFB2(fd, framebuffer);
    if (!fb)
        return errno ? -errno : -EIO;

    uint64_t length = (uint64_t)fb->pitches[0] * fb->height;
    int result = -EOPNOTSUPP;
    if (fb->pixel_format == DRM_FORMAT_XRGB8888 && fb->handles[0] &&
        !fb->handles[1] && !fb->offsets[0] && fb->width && fb->height &&
        fb->pitches[0] >= (uint64_t)fb->width * 4 && length <= UINT32_MAX &&
        (!(fb->flags & DRM_MODE_FB_MODIFIERS) || fb->modifier == DRM_FORMAT_MOD_LINEAR)) {
        uint64_t offset;
        result = drmModeMapDumbBuffer(fd, fb->handles[0], &offset);
        if (!result) {
            *out = (struct initium_drm_mapping){
                offset, fb->width, fb->height, fb->pitches[0], length, fb->handles[0]
            };
        }
    }

    for (unsigned int i = 0; i < 4; ++i) {
        if (!fb->handles[i])
            continue;
        unsigned int previous = 0;
        while (previous < i && fb->handles[previous] != fb->handles[i])
            ++previous;
        if (previous == i && (result || i != 0))
            drmCloseBufferHandle(fd, fb->handles[i]);
    }
    drmModeFreeFB2(fb);
    return result;
}
