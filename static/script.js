
async function upload_file(file) {
    console.log(file);
    var xmlHttpRequest = new XMLHttpRequest();

    console.log(file);
    var mimeType = "image/png";
    const fileName = file.name;

    xmlHttpRequest.open('POST', '/upload', true);
    xmlHttpRequest.setRequestHeader('Content-Type', mimeType);
    xmlHttpRequest.setRequestHeader('Content-Disposition', 'attachment; filename="' + fileName + '"');
    xmlHttpRequest.send(file);

    xmlHttpRequest.onload = function () {
        if (xmlHttpRequest.readyState === xmlHttpRequest.DONE) {
            if (xmlHttpRequest.status === 200) {
                console.log(xmlHttpRequest.response);
                console.log(xmlHttpRequest.responseText);
            }
        }
    };
}

class ImageHandler {
    constructor(parent, element, image, handles) {
        this.element = element;
        this.parent = parent;
        this.init();

        this.anim = this.do_resize.bind(this);

        this.set_element_style();

        dragger(handles[0], this.delta_left.bind(this), this.delta_top.bind(this));
        dragger(handles[1], this.delta_right.bind(this), this.delta_top.bind(this));
        dragger(handles[2], this.delta_right.bind(this), this.delta_bottom.bind(this));
        dragger(handles[3], this.delta_left.bind(this), this.delta_bottom.bind(this));

        if (image) this.set_image(image);
        requestAnimationFrame(this.anim);
    }

    init() {
        this.should_resize = true;

        this.orig = {
            top: 0,
            left: 0,
            width: 1,
            height: 1
        };

        this.pos = {
            top: 0,
            left: 0,
            width: 1,
            height: 1
        };
    }

    set_image(image, data_url = null, raw_file = null) {
        if (!data_url) data_url = image_to_data_url(image);
        this.init();

        this.raw_file = raw_file;
        this.data_url = data_url;
        this.image = image;
        this.parent.style["background-image"] = `url("${data_url}")`
        this.should_resize = true;
    }

    set_element_style() {
        this.element.style.top = this.pos.top + "px";
        this.element.style.left = this.pos.left + "px";
        this.element.style.width = this.pos.width + "px";
        this.element.style.height = this.pos.height + "px";
    }

    resize() {
        this.should_resize = true;
    }

    do_resize() {
        // if (!this.image) return;
        if (!this.should_resize || !this.image) {
            requestAnimationFrame(this.anim);
            return;
        }
        this.should_resize = false;

        const box = this.parent.getBoundingClientRect();
        let image_aspect = this.image.naturalHeight / this.image.naturalWidth;
        const [top, left, width, height] = calculate_image_background_box(box, image_aspect);

        // Fast and dirty zero check
        const rule_three = (orig, current, value) => (orig && current && value) ? current / orig * value : 0;

        this.pos.top = rule_three(this.orig.height, height, this.pos.top - this.orig.top) + top;
        this.pos.left = rule_three(this.orig.width, width, this.pos.left - this.orig.left) + left;
        this.pos.width = rule_three(this.orig.width, width, this.pos.width);
        this.pos.height = rule_three(this.orig.height, height, this.pos.height);

        this.orig = {
            top: top,
            left: left,
            width: width,
            height: height
        };
        requestAnimationFrame(this.anim);
    }

    delta_left(dy) {
        this.pos.left -= dy;
        this.pos.width += dy;

        this.element.style.left = this.pos.left + "px";
        this.element.style.width = this.pos.width + "px";
    }

    delta_right(dy) {
        this.pos.width -= dy;
        this.element.style.width = this.pos.width + "px";
    }

    delta_top(dy) {
        this.pos.top -= dy;
        this.pos.height += dy;

        this.element.style.top = this.pos.top + "px";
        this.element.style.height = this.pos.height + "px";
    }

    delta_bottom(dy) {
        this.pos.height -= dy;
        this.element.style.height = this.pos.height + "px";
    }

    crop(f = null) {
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d");

        const [left, top, width, height] = [
            this.pos.left - this.orig.left,
            this.pos.top - this.orig.top,
            this.pos.width,
            this.pos.height
        ]

        const zoomWidth = this.image.naturalWidth / this.orig.width;
        const zoomHeight = this.image.naturalHeight / this.orig.height;

        canvas.width = width * zoomWidth;
        canvas.height = height * zoomHeight;

        ctx.drawImage(this.image,
            left * zoomWidth, top * zoomHeight, width * zoomWidth, height * zoomHeight,
            0, 0, width * zoomWidth, height * zoomHeight
        );

        if (f) canvas.toBlob(f)
        else return canvas.toDataURL();

        // return canvas.toDataURL();
    }

    async upload() {
        if (!this.image) return;

        this.crop(upload_file);
        // if (!this.data_url && this.image) this.data_url = image_to_data_url(this.image);
        // if (!this.data_url) return;

        // upload_file(data_url);
    }
}

function image_to_data_url(image) {
    if (image instanceof HTMLImageElement) {
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d");

        canvas.width = image.naturalWidth;
        canvas.height = image.naturalHeight;

        ctx.drawImage(image, 0, 0);
        return canvas.toDataURL("image/jpeg");
    } else {
        return URL.createObjectURL(image);
    }
}

function calculate_image_background_box(box, aspect) {
    let width, height;

    if (box.width * aspect > box.height) {
        height = box.height;
        width = box.height / aspect;
    } else {
        width = box.width;
        height = box.width * aspect;
    }

    const top = (box.height - height) / 2;
    const left = (box.width - width) / 2;

    return [top, left, width, height];
}

const elements = {};
let image_handler = null;
async function main() {
    ["handles", "handle0", "handle1", "handle2", "handle3", "content", "myFileInput", "target"].forEach(x => elements[x] = document.getElementById(x));
    elements.myFileInput.addEventListener('change', setPicToManipulate, false);

    const handles = [...Array(4).keys()].map(i => elements["handle" + i]);
    image_handler = new ImageHandler(elements["content"], elements["handles"], null, handles);
    window.addEventListener("resize", image_handler.resize.bind(image_handler));

    const image = new Image();
    image.onload = () => image_handler.set_image(image);
    image.src = "test.jpg";
}


function setPicToManipulate() {
    const file = elements.myFileInput.files[0];
    console.log(file);
    if (file) {
        const image = new Image();
        // This feel unnecessary
        image.onload = () => image_handler.set_image(image);
        image.src = URL.createObjectURL(file)
    }
}

function sendPic() {
    var file = elements.myFileInput.files[0];
    const s = {};

    for (let field in file) {
        console.log(field);
        s[field] = file[field];
    }

    upload_file(file).then(x => elements.textbox.innerHTML += x);

    // Send file here either by adding it to a `FormData` object
    // and sending that via XHR, or by simply passing the file into
    // the `send` method of an XHR instance.
}

// const image = new Image(); // Using optional size for image
// image.onload = drawImageActualSize; // Draw when image has loaded

// // Load an image of intrinsic size 300x227 in CSS pixels
// image.src = 'test.jpg';

function drawImageActualSize() {
    // Use the intrinsic size of image in CSS pixels for the canvas element
    const w = this.naturalWidth;
    const h = this.naturalHeight;

    const aspect = w / h;


    drag = dragger((ctx, width, height, zoom, offsetx, offsety) => {
        if (ctx instanceof CanvasRenderingContext2D) {
            console.log(width, height, zoom, offsetx, offsety)
            if (width < height * aspect) {
                ctx.drawImage(this, 0, 0, width, width / aspect)
            } else {
                ctx.drawImage(this, 0, 0, height * aspect, height)
            }
        }
    });

    drag.start();

    crop = cropper(elements.canvas2, this);

    crop.start();
}

function capture_img() {
    const data = image_handler.crop();
    elements["target"].src = data;

    image_handler.upload();

    // console.log("Capturing image")
    // const canvas = drag.getCanvas();
    // var dataURL = canvas.toDataURL();

    // elements.ref_img.src = dataURL;
}


window.addEventListener('load', main);

// elements.ref_img.addEventListener('load', e => {
//     drag.force_draw()
//     // ctx.drawImage(image, 33, 71, 104, 124, 21, 20, 87, 104);
// });
