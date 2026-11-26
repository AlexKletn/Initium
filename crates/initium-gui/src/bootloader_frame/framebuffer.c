#include <errno.h>
#include <linux/fb.h>
#include <stdint.h>
#include <sys/ioctl.h>

struct framebuffer_layout {
    uint32_t width, height, x_offset, y_offset, stride, length;
};

int initium_framebuffer_layout(int fd, struct framebuffer_layout *out)
{
    struct fb_fix_screeninfo fixed;
    struct fb_var_screeninfo variable;

    if (ioctl(fd, FBIOGET_FSCREENINFO, &fixed) < 0 ||
        ioctl(fd, FBIOGET_VSCREENINFO, &variable) < 0)
        return -errno;

    if (variable.bits_per_pixel != 32 ||
        variable.red.offset != 16 || variable.red.length != 8 || variable.red.msb_right ||
        variable.green.offset != 8 || variable.green.length != 8 || variable.green.msb_right ||
        variable.blue.offset != 0 || variable.blue.length != 8 || variable.blue.msb_right ||
        fixed.type != FB_TYPE_PACKED_PIXELS || fixed.visual != FB_VISUAL_TRUECOLOR ||
        variable.nonstd || (variable.vmode & FB_VMODE_YWRAP))
        return -EOPNOTSUPP;

    *out = (struct framebuffer_layout){
        variable.xres, variable.yres, variable.xoffset, variable.yoffset,
        fixed.line_length, fixed.smem_len
    };
    return 0;
}
