
async function upload_file(file) {
    var xmlHttpRequest = new XMLHttpRequest();

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

        const left_f = this.delta_left.bind(this);
        const right_f = this.delta_right.bind(this);
        const top_f = this.delta_top.bind(this);
        const bot_f = this.delta_bottom.bind(this);


        new Dragger(handles[0], left_f, top_f, 0);
        new Dragger(handles[1], right_f, top_f, 1);
        new Dragger(handles[2], right_f, bot_f, 2);
        new Dragger(handles[3], left_f, bot_f, 3);

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

        console.log("HERE");

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

        this.delta_left(0);
        this.delta_top(0);

        this.delta_right(0);
        this.delta_bottom(0);

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

        // this.crop(upload_file);
        // if (!this.data_url && this.image) this.data_url = image_to_data_url(this.image);
        // if (!this.data_url) return;

        // upload_file(data_url);

        return { "Success": [{ "name": "TL ALP DRINK AMAND", "price": 2.99 }, { "name": "800G QUAKER HAVERM |", "price": 2.95 }, { "name": "VUILZAK GROEN 30L", "price": 11.1 }, { "name": "KOMKOMMER", "price": 0.96 }, { "name": "1.6. ICE TEA ZERO", "price": 2.11 }, { "name": "O.5L MONST ENERGY", "price": 1.42 }, { "name": "SOCL MNSTR PARADIS", "price": 1.42 }, { "name": "BLADPETERSELTIE VER", "price": 1.69 }, { "name": "ELVEA TOMATENCONCE |", "price": 2.29 }, { "name": "SOCL MONSTER MU/GT", "price": 1.42 }, { "name": "VOEDING |", "price": 73.99 }, { "name": "PILION FETA 200G", "price": 1.66 }, { "name": "206 DLL KORTANDER \u0014", "price": 0.99 }] }
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
    if (file) {
        const image = new Image();
        // This feel unnecessary
        image.onload = () => image_handler.set_image(image);
        image.src = URL.createObjectURL(file)
    }
}


async function capture_img() {
    // const data = image_handler.crop();
    // elements["target"].src = data;

    const {Success: result} = await image_handler.upload();
    console.log(result);

}


window.addEventListener('load', main);
