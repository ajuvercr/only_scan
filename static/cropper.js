
const cropper = function (canvas, image, options = {}) {
    /** @type {CanvasRenderingContext2D} */
    const ctx = canvas.getContext('2d');

    let width, height = 0;
    let b_width, b_height, b_x, b_y = 0;
    let image_width, image_height = 0;
    let inited = false;
    let changed = true;

    const handle_size = 10;

    function init() {
        set_sizing();

        inited = true;
        b_width = width * 0.8;
        b_x = width * 0.1;
        b_height = height * 0.8;
        b_y = height * 0.1;
    }

    function set_sizing() {
        let box = canvas.getBoundingClientRect();
        width = box.width;
        height = box.height;
        canvas.width = width;
        canvas.height = height;

        const w = options.width || image.naturalWidth;
        const h = options.height || image.naturalHeight;

        const aspect = w / h;
        if (width < height * aspect) {
            image_width = width;
            image_height = width / aspect;
        } else {
            image_width = height * aspect;
            image_height = height;
        }
    }

    const circle = (cx, cy) => {
        ctx.beginPath();
        ctx.arc(cx, cy, handle_size, 0, 2 * Math.PI);
        ctx.fill();
    }


    function do_draw() {
        set_sizing();

        ctx.clearRect(0, 0, width, height);
        ctx.globalAlpha = 1;

        ctx.drawImage(image, 0, 0, image_width, image_height);

        console.log("here", b_x, b_y, b_height, b_height);
        ctx.fillStyle = "white";
        circle(b_x, b_y);
        circle(b_x, b_y + b_height);
        circle(b_x + b_width, b_y);
        circle(b_x + b_width, b_y + b_height);

        ctx.globalAlpha = 0.8;
        ctx.fillRect(b_x, b_y, b_width, b_height);
    }

    function draw() {
        if (!inited) {
            init();
        }
        if (changed) {
            changed = false;
            do_draw();
        }

        requestAnimationFrame(draw);
    }

    this.start = () => draw();

    return this;
}
