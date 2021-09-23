
async function upload_file(file) {
    var xmlHttpRequest = new XMLHttpRequest();

    var fileName = file.name;
    var mimeType = "plain";

    xmlHttpRequest.open('POST', '/upload', true);
    xmlHttpRequest.setRequestHeader('Content-Type', mimeType);
    xmlHttpRequest.setRequestHeader('Content-Disposition', 'attachment; filename="' + fileName + '"');
    xmlHttpRequest.send(file);

    xmlHttpRequest.onload = function () {
        if (xmlHttpRequest.readyState === xmlHttpRequest.DONE) {
            if (xmlHttpRequest.status === 200) {
                console.log(xmlHttpRequest.response);
                console.log(xmlHttpRequest.responseText);

                elements.textbox.innerHTML += xmlHttpRequest.responseText;
            }
        }
    };
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
async function main() {
    ["handles", "handle0", "handle1", "handle2", "handle3", "content", "myFileInput", "target"].forEach(x => elements[x] = document.getElementById(x));
    elements.myFileInput.addEventListener('change', setPicToManipulate, false);

    const image = new Image();

    image.onload = function () {
        const box = elements["content"].getBoundingClientRect();
        let image_aspect = this.naturalHeight / this.naturalWidth;
        const [top, left, width, height] = calculate_image_background_box(box, image_aspect);

        const rect = elements["handles"];
        rect.style.top = top + "px";
        rect.style.left = left + "px";
        rect.style.width = width + "px";
        rect.style.height = height + "px";

        rect.__mu = {};
        rect.__mu.top = top;
        rect.__mu.left = left;

        rect.__mu.orig_top = top;
        rect.__mu.orig_left = left;

        rect.__mu.width = width;
        rect.__mu.height = height;

        rect.__mu.orig_width = width;
        rect.__mu.orig_height = height;

        setTimeout(() => {
            do_crop(rect, image);
        }, 2000)
    };

    image.src = "test.jpg";
    setup_handles();

}

function do_crop(rect, image) {
    const canvas = document.createElement("canvas");
    const ctx = canvas.getContext("2d");


    const [left, top, width, height] = [
        rect.__mu.left - rect.__mu.orig_left,
        rect.__mu.top - rect.__mu.orig_top,
        rect.__mu.width,
        rect.__mu.height
    ]

    const zoomWidth = image.naturalWidth / rect.__mu.orig_width;
    const zoomHeight = image.naturalHeight / rect.__mu.orig_height;

    canvas.width = width * zoomWidth;
    canvas.height = height * zoomHeight;

    ctx.drawImage(image,
        left * zoomWidth, top * zoomHeight, width * zoomWidth, height * zoomHeight,
        0, 0, width * zoomWidth, height * zoomHeight
    );


    canvas.toDataURL();

    setTimeout(
        () => elements["target"].src = canvas.toDataURL(),
        100
    );
}


function setup_handles() {
    const handles = [...Array(4).keys()].map(i => elements["handle" + i]);

    const rect = elements["handles"];

    if (!rect.__mu) rect.__mu = {};

    const delta_left = (dy) => {
        rect.__mu.left -= dy;
        rect.__mu.width += dy;

        rect.style.left = rect.__mu.left + "px";
        rect.style.width = rect.__mu.width + "px";
    };

    const delta_right = (dy) => {
        rect.__mu.width -= dy;
        rect.style.width = rect.__mu.width + "px";
    };

    const delta_top = (dy) => {
        rect.__mu.top -= dy;
        rect.__mu.height += dy;

        rect.style.top = rect.__mu.top + "px";
        rect.style.height = rect.__mu.height + "px";
    };

    const delta_bottom = (dy) => {
        rect.__mu.height -= dy;
        rect.style.height = rect.__mu.height + "px";
    };

    dragger(handles[0], delta_left, delta_top);
    dragger(handles[1], delta_right, delta_top);
    dragger(handles[2], delta_right, delta_bottom);
    dragger(handles[3], delta_left, delta_bottom);
}

function setPicToManipulate() {
    var file = elements.myFileInput.files[0];
    if (file)
        elements.content.style["background-image"] = `url("${URL.createObjectURL(file)}")`
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

// let drag = undefined;
// let crop = undefined;

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
    console.log("Capturing image")
    const canvas = drag.getCanvas();
    var dataURL = canvas.toDataURL();

    elements.ref_img.src = dataURL;
}


window.addEventListener('load', main);

// elements.ref_img.addEventListener('load', e => {
//     drag.force_draw()
//     // ctx.drawImage(image, 33, 71, 104, 124, 21, 20, 87, 104);
// });
