import * as d3 from "d3";

type SVG = d3.Selection<SVGGElement, unknown, HTMLElement, any>;
type Row = {
    date: Date;
    flag: string;
    amount: number;
    account: string;
    from: string[];
};

type Seg = {
    data: [string, Row][];
    nested: { [label: string]: Seg };
    parent?: Seg;
};


class SumBuffer {
    private readonly size: number;
    private at: number = 0;
    private readonly buffer: number[];
    private total: number;

    constructor(size: number) {
        this.size = size;
        this.buffer = new Array(size).map(() => 0);
    }

    push(item: number) {
        this.buffer[this.at] = item;
        this.total += item;

        this.at++;
        if (this.at === this.size) this.at = 0;
        this.total -= this.buffer[this.at];
    }

    sum(): number {
        return this.total;
    }
}

var parseDate = d3.timeParse("%Y-%m-%d");

function conversor(d: any): Row {
    d.amount = +d.number;
    d.account = d.account.trim();
    d.from = d.account.split(":");
    d.date = parseDate(d.date);
    return d;
}


function getInput(loc: string, body?: ReadableStream): Promise<Seg> {
    return new Promise((res) => {
        const data: Seg = { data: [], nested: {} };

        const reqInit = body == undefined ? {} : { method: "POST", body };

        d3.csv(loc, reqInit, async (_data: any) => {
            const row = conversor(_data);
            let current = data;
            for (let f of row.from) {
                current.data.push([f, row]);
                if (!current.nested[f])
                    current.nested[f] = { data: [], nested: {}, parent: current };
                current = current.nested[f];
            }
            current.data.push(["", row]);
        }).then(() => res(data));
    });
}

const timePerDay = 1000 * 60 * 60 * 24;

var margin = { top: 10, right: 60, bottom: 30, left: 60 },
    width = 1200 - margin.left - margin.right,
    height = 800 - margin.top - margin.bottom;

// append the svg object to the body of the page
function getSvg(id: string) {
    // append the svg object to the body of the page
    return d3
        .select(id)
        .append("svg")
        .attr("width", width + margin.left + margin.right)
        .attr("height", height + margin.top + margin.bottom)
        .append("g")
        .attr("transform", "translate(" + margin.left + "," + margin.top + ")");
}

// color palette
const colors = [
    "#e41a1c",
    "#377eb8",
    "#4daf4a",
    "#984ea3",
    "#ff7f00",
    "#ffff33",
    "#a65628",
    "#f781bf",
    "cyan"
];


function setupInfo(svg: SVG) {
    // Create the circle that travels along the curve of chart
    var focus = svg
        .append('g')
        .append('circle')
        .attr("class", "info")
        .style("fill", "none")
        .attr("stroke", "black")
        .attr('r', 8.5)
        .style("opacity", 0)

    // Create the text that travels along the curve of chart
    var focusText = svg
        .append('g')
        .append('text')
        .attr("class", "info")
        .style("opacity", 0)
        .style('z-index', 5)
        .attr("text-anchor", "middle")
        .attr("alignment-baseline", "middle")

    return [focus, focusText];
}

function drawAndGetScales(svg: SVG, stats: [string, [number, number][]][], monthC: number, startDate: Date) {
    console.log("Day count", monthC);
    const x = d3
        .scaleLinear()
        .domain([0, monthC])
        .range([0, width]);

    svg
        .append("g")
        .attr("transform", `translate(0, ${height})`)
        .call(d3.axisBottom(x).ticks(10).tickFormat(d => {
            const date = new Date(startDate.getTime() + d.valueOf() * timePerDay);
            return date.toLocaleDateString("nl-BE");
        }));

    // Add Y axis
    const maxV = d3.max(stats, d => d3.max(d[1], d => d[1]));
    const minV = d3.min(stats, d => d3.min(d[1], d => d[1]));
    const y = d3.scaleLinear().domain([minV, maxV]).range([height, 0]);
    svg.append("g").call(d3.axisLeft(y));
    return [x, y];
}

function drawLegend(svg: SVG, stats: [string, [number, number][]][]) {
    // Add one dot in the legend for each name.
    svg.selectAll("mydots")
        .data(stats)
        .enter()
        .append("circle")
        .attr("cx", width - 170)
        .attr("cy", function(_d: any, i: number) { return 14 + i * 25 }) // 100 is where the first dot appears. 25 is the distance between dots
        .attr("r", 7)
        .style("fill", function(_d: any, i: number) { return colors[i] })
    svg.selectAll("mylabels")
        .data(stats)
        .enter()
        .append("text")
        .attr("x", width - 150)
        .attr("y", function(_d: any, i: number) { return 20 + i * 25 }) // 100 is where the first dot appears. 25 is the distance between dots
        .style("fill", function(_d: any, i: number) { return colors[i] })
        .text(function(d) { return d[0] })
        .attr("text-anchor", "left")
        .style("alignment-baseline", "middle")
}

type Scale = d3.ScaleLinear<number, number, never>;
function drawLines(svg: SVG, stats: [string, [number, number][]][], x: Scale, y: Scale) {
    const drawLine = d3.line().x((d, i) => x(i)).y(d => y(d[1]));
    console.log(stats[0])
    svg
        .selectAll(".line")
        .data(stats)
        .join("path")
        .attr("tooltip", d => d[0])
        .attr('pointer-events', 'visibleStroke')
        .attr("fill", "none")
        .attr("stroke", (_d, i) => colors[i])
        .attr("stroke-width", 3)
        .attr("d", d => drawLine(d[1]))
}

function getStartAndEndDate(data: [string, Row][]) {
    const sumstat = d3.group(data, (x) => x[0])

    const endDate = Math.max(...Array.from(sumstat.values()).flat().map(x => x[1].date.getTime()));
    const startDate = Math.min(...Array.from(sumstat.values()).flat().map(x => x[1].date.getTime()));
    return [startDate, endDate];
}

let currentStartDate = 0;
let currentEndDate = 0;
export async function setupGraphs(location: string, svgContainerId: string, helpId: string, parentId: string, samplesPerDayId: string, startDateId: string, endDateId: string, body?: ReadableStream) {
    const data = await getInput(location, body);
    const svg = getSvg(svgContainerId);
    const help = d3.select(helpId);
    const parentButton = d3.select(parentId);

    const samplePerDayField = <HTMLInputElement>document.getElementById(samplesPerDayId);

    let daysPerSample = parseInt(samplePerDayField.value);
    samplePerDayField.addEventListener("input", d => {
        const target = d.target as HTMLInputElement;
        daysPerSample = parseInt(target.value);
            updateLocalStat(current.data);
        update(new Date(currentStartDate), currentEndDate);
    });

    const [ultimateStartDate, ultimateEndDate] = getStartAndEndDate(data.data);
    currentStartDate = ultimateStartDate;
    currentEndDate = ultimateEndDate;

    // console.log(startDateId, endDateId);
    if (startDateId && endDateId) {
        const delta = ultimateEndDate - ultimateStartDate;

        const startDateSlider = <HTMLInputElement>document.getElementById(startDateId);
        const endDateSlider = <HTMLInputElement>document.getElementById(endDateId);

        const updateThings = () => {
            const procentStart = parseInt(startDateSlider.value) / 100;
            const procentEnd = parseInt(endDateSlider.value) / 100;

            const start = new Date(ultimateStartDate + Math.floor(delta * procentStart));
            const end = new Date(ultimateStartDate + Math.floor(delta * procentEnd));

            currentStartDate = start.getTime();
            currentEndDate = end.getTime();


            update(start, end.getTime());
        };

        startDateSlider.addEventListener("input", updateThings);
        endDateSlider.addEventListener("input", updateThings);

        startDateSlider.min = "" + 0;
        startDateSlider.max = "" + 100;
        endDateSlider.min = "" + 0;
        endDateSlider.max = "" + 100;

        startDateSlider.value = "" + 0;
        endDateSlider.value = "" + 100;
    }


    let localStat: [string, number[]][] = [];

    function updateLocalStat(data: [string, Row][]) {
        const sumstat = d3.group(data, (x) => x[0])
        const dayCount = Math.ceil((ultimateEndDate - ultimateStartDate) / timePerDay);

        localStat = Array.from(sumstat.entries()).map(x => {

            // Buffer with values per time value
            const sumBuffer = Array.from({ length: dayCount + 1 }).map(() => 0);
            const valuePerDay = Array.from({ length: dayCount + 1 }).map(() => 0)

            x[1].forEach(([_, x]) => {
                const day = Math.floor((x.date.getTime() - ultimateStartDate) / timePerDay);
                if (day < dayCount && day >= 0)
                    valuePerDay[day] += x.amount;
            });


            for (let i = 0; i < dayCount; i++) {
                sumBuffer[i] = (sumBuffer[i - 1] || 0) + valuePerDay[i] - (valuePerDay[i - daysPerSample] || 0);
            }

            return [x[0], sumBuffer]
        });
    }

    function update(startDate: Date, endDate: number) {
        svg.selectAll("*").remove();

        // Number of samples
        const dayCount = Math.ceil((endDate - startDate.getTime()) / timePerDay);

        const startIndex = Math.floor((startDate.getTime() - ultimateStartDate) / timePerDay);
        const endIndex = Math.floor((endDate - ultimateStartDate) / timePerDay);
        const stats: [string, [number, number][]][] = localStat.map(x => [x[0], x[1].slice(startIndex, endIndex).map((x, i) => [i, x])]);

        const [x, y] = drawAndGetScales(svg, stats, dayCount, startDate);

        const bisect = d3.bisector((d: [number, number]) => d[0]).center;

        svg
            .append('rect')
            .style("fill", "none")
            .style("pointer-events", "all")
            .attr('width', width)
            .attr('height', height)
            .on('click', mouseclick)
            .on('mouseover', mouseover)
            .on('mousemove', mousemove)
            .on('mouseout', mouseout);

        drawLines(svg, stats, x, y);

        drawLegend(svg, stats);

        const [focus, focusText] = setupInfo(svg);
        // What happens when the mouse move -> show the annotations at the right positions.
        function mouseover() {
            focus.style("opacity", 1)
            focusText.style("opacity", 1)
        }

        let lastSelectedLine = -1;
        function mouseclick(e: MouseEvent) {
            if (lastSelectedLine != -1) {
                const tooltip = stats[lastSelectedLine][0];
                next(tooltip);
            }
        }

        function mousemove(e: MouseEvent) {
            // recover coordinate we need
            const [mouse_x, mouse_y] = d3.pointer(e);
            const x0 = x.invert(mouse_x);
            const y0 = y.invert(mouse_y);


            let selectedData = [0, 0];
            let selectedLine = -1;
            let distance = 100000000000000;

            for (let lineIndex = 0; lineIndex < stats.length; lineIndex++) {

                const i = bisect(stats[lineIndex][1], x0, 1);

                const startLocation = stats[lineIndex][1][i - 1];
                const endLocation = stats[lineIndex][1][i];

                if (!endLocation || !startLocation) continue;

                const dx = (endLocation[0] - startLocation[0]);
                const dy = (endLocation[1] - startLocation[1]);


                const percentage = (x0 - startLocation[0]) / dx;
                const selectX = startLocation[0] + percentage * dx;
                const selectY = startLocation[1] + percentage * dy;

                const yDelta = Math.abs(selectY - y0)
                if (yDelta < distance) {
                    selectedLine = lineIndex;
                    distance = yDelta;
                    selectedData = [selectX, selectY];
                }
            }

            updateSelectedLine(selectedLine);

            // const selectedData = stats[0][1][i-1];
            (<d3.Selection<SVGCircleElement, unknown, HTMLElement, any>>
                focus.attr("cx", x(selectedData[0])))
                .attr("cy", y(selectedData[1]));

            (<d3.Selection<SVGTextElement, unknown, HTMLElement, any>>focusText
                .html("" + selectedData[1].toFixed(2)))
                .attr("x", x(selectedData[0]))
                .attr("y", y(selectedData[1]) + 30)
        }


        function updateSelectedLine(next: number) {
            if (next == lastSelectedLine) return;

            const tooltip = stats[next][0] || "root";

            // set info
            help.text(tooltip);

            // set fatty
            document.querySelectorAll(`[tooltip='${tooltip}']`).forEach(i => i.classList.add("hover"));

            if (lastSelectedLine != -1) {
                const tooltip = stats[lastSelectedLine][0];
                document.querySelectorAll(`[tooltip='${tooltip}']`).forEach(i => i.classList.remove("hover"));
            }

            lastSelectedLine = next;
        }

        function mouseout() {
            focus.style("opacity", 0)
            focusText.style("opacity", 0)
            if (lastSelectedLine != -1) {
                const tooltip = stats[lastSelectedLine][0];
                document.querySelectorAll(`[tooltip='${tooltip}']`).forEach(i => i.classList.remove("hover"));
            }
            lastSelectedLine = -1;
        }
    }

    let current = data;

    function next(target: string) {
        if (!target) return;
        current = current.nested[target];
        updateLocalStat(current.data);
        update(new Date(currentStartDate), currentEndDate);
    }

    function parent() {
        if (current.parent) {
            current = current.parent;
            updateLocalStat(current.data);
            update(new Date(currentStartDate), currentEndDate);
        }
    }

    parentButton.on("click", parent);

    updateLocalStat(data.data);
    update(new Date(currentStartDate), currentEndDate);
}

