const ACTIONS = {
    DOWN: "down",
    UP: "up",
    MOVE: "move",
}

function getEventLocation(e) {
    if (e.touches && e.touches.length == 1) {
        return [e.touches[0].clientX, e.touches[0].clientY];
    }
    else if (e.clientX && e.clientY) {
        return [e.clientX, e.clientY];
    }
}

class Dragger {
    constructor(element, horizontal, vertical, meta = null) {
        this.hor_f = horizontal;
        this.vert_f = vertical;

        this.is_dragging = false;

        this.meta = meta;
        this.last_x = 0;
        this.last_y = 0;

        element.addEventListener('mousedown', this.onPointerDown.bind(this));

        element.addEventListener('touchstart', (e) => {
            this.handleTouch(e, ACTIONS.DOWN)
        })
        document.addEventListener('mouseup', this.onPointerUp.bind(this))
        element.addEventListener('touchend', (e) => {
            this.handleTouch(e, ACTIONS.UP)
        });
        element.addEventListener('touchcancel', (e) => {
            this.handleTouch(e, ACTIONS.UP)
        });
        document.addEventListener('mousemove', this.onPointerMove.bind(this))
        element.addEventListener('touchmove', (e) => {
            this.handleTouch(e, ACTIONS.MOVE)
        })

    }

    onPointerDown(e) {
        this.is_dragging = true;

        const [x, y] = getEventLocation(e);

        this.last_x = x;
        this.last_y = y;
    }

    onPointerMove(e) {
        function abs(x) {
            return x < 0 ? -1 * x : x;
        }

        if (this.is_dragging) {
            const [x, y] = getEventLocation(e);

            if(abs(this.last_x - x) + abs(this.last_y - y) > 100) return false;

            if (this.last_x - x != 0) {
                this.hor_f(this.last_x - x);
            }

            if (this.last_y - y != 0) {
                this.vert_f(this.last_y - y);
            }

            this.last_x = x;
            this.last_y = y;
            return true;
        }
        return false;
    }

    onPointerUp(e) {
        this.is_dragging = false
    }

    handleTouch(e, action) {
        if (e.touches.length  > 0) {
            switch (action) {
                case ACTIONS.DOWN:
                    e.preventDefault();
                    this.onPointerDown(e); break;

                case ACTIONS.UP:
                    e.preventDefault();
                    this.onPointerUp(e); break;

                case ACTIONS.MOVE:
                    if(this.onPointerMove(e))
                    e.preventDefault();
                    break;
            }
        }
    }
}
