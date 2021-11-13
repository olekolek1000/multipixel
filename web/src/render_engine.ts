import * as glMatrix from "gl-matrix";

const text_vs_standard = `
	attribute vec3 vertpos;
	attribute vec2 vertuv;
	uniform mat4 P;
	uniform mat4 M;
	varying lowp vec2 UV;
	void main(void) {
		gl_Position = P * M * vec4(vertpos, 1.0);
		UV = vertuv;
	}
`;

const text_fs_solid = `
	precision highp float;
	varying vec2 UV;

	uniform sampler2D tex;

	void main() {
		gl_FragColor = texture2D(tex, UV);
	}
`;

function loadShader(gl: WebGL2RenderingContext, type: number, source: string) {
	const shader = gl.createShader(type);
	gl.shaderSource(shader, source);
	gl.compileShader(shader);

	if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
		alert('An error occurred compiling the shaders: ' + gl.getShaderInfoLog(shader));
		gl.deleteShader(shader);
		return null;
	}

	return shader;
}

function initShaderProgram(gl: WebGL2RenderingContext, source_vs: string, source_fs: string) {
	const vertexShader = loadShader(gl, gl.VERTEX_SHADER, source_vs);
	const fragmentShader = loadShader(gl, gl.FRAGMENT_SHADER, source_fs);

	const shaderProgram = gl.createProgram();
	gl.attachShader(shaderProgram, vertexShader);
	gl.attachShader(shaderProgram, fragmentShader);
	gl.linkProgram(shaderProgram);
	if (!gl.getProgramParameter(shaderProgram, gl.LINK_STATUS)) {
		alert('Unable to initialize the shader program: ' + gl.getProgramInfoLog(shaderProgram));
		return null;
	}

	return shaderProgram;
}

function initBufferQuad(gl: WebGL2RenderingContext) {
	let vao = gl.createVertexArray();
	gl.bindVertexArray(vao);

	const positionBuffer = gl.createBuffer();
	gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
	const positions = [
		0.0, 0.0,
		0.0, 1.0,
		1.0, 1.0,
		0.0, 0.0,
		1.0, 1.0,
		1.0, 0.0
	];

	gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(positions), gl.STATIC_DRAW);
	gl.enableVertexAttribArray(0);
	gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

	const uvs = [
		0.0, 0.0,
		0.0, 1.0,
		1.0, 1.0,
		0.0, 0.0,
		1.0, 1.0,
		1.0, 0.0
	];

	const uvBuffer = gl.createBuffer();
	gl.bindBuffer(gl.ARRAY_BUFFER, uvBuffer);
	gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(uvs), gl.STATIC_DRAW);
	gl.enableVertexAttribArray(1);
	gl.vertexAttribPointer(1, 2, gl.FLOAT, false, 0, 0);

	return {
		vao: vao,
		position: positionBuffer,
		uv: uvBuffer,
	};
}

class Shader {
	program;
	uniform_p;
	uniform_m;

	constructor(gl: WebGL2RenderingContext, source_vs: string, source_fs: string) {
		this.program = initShaderProgram(gl, source_vs, source_fs);
		this.uniform_p = this.createUniform(gl, "P");
		this.uniform_m = this.createUniform(gl, "M");
	}

	createUniform = function (gl: WebGL2RenderingContext, name: string) {
		return gl.getUniformLocation(this.program, name);
	}

	bind = function (gl: WebGL2RenderingContext) {
		gl.useProgram(this.program);
	}

	setM = function (gl: WebGL2RenderingContext, m: any) {
		gl.uniformMatrix4fv(this.uniform_m, false, m);
	}

	setP = function (gl: WebGL2RenderingContext, p: any) {
		gl.uniformMatrix4fv(this.uniform_p, false, p);
	}
}

export class Texture {
	texture: WebGLTexture;
	width: number;
	height: number;
}

export class RenderEngine {
	canvas: HTMLCanvasElement;
	gl: WebGL2RenderingContext;

	buffer_quad;
	shader;
	projection;

	constructor(canvas: HTMLCanvasElement) {
		this.canvas = canvas;
		this.gl = canvas.getContext("webgl2",
			{
				antialias: false,
				alpha: false,
				premultipliedAlpha: false
			});

		if (!this.gl) {
			alert("WebGL not supported");
			return;
		}

		this.buffer_quad = initBufferQuad(this.gl);
		this.shader = new Shader(this.gl, text_vs_standard, text_fs_solid);

		this.projection = glMatrix.mat4.create();

		this.gl.disable(this.gl.DEPTH_TEST);
		this.gl.enable(this.gl.BLEND);
		this.gl.blendFunc(this.gl.SRC_ALPHA, this.gl.ONE_MINUS_SRC_ALPHA);
	}

	clear = function (r: number, g: number, b: number, a: number) {
		this.gl.clearColor(r, g, b, a);
		this.gl.clear(this.gl.COLOR_BUFFER_BIT);
	}

	viewportFullscreen = function () {
		this.gl.viewport(0, 0, this.canvas.width, this.canvas.height);
	}

	setOrtho = function (left: number, right: number, bottom: number, top: number) {
		glMatrix.mat4.ortho(this.projection, left, right, bottom, top, -1.0, 1.0);
	}

	getCanvas = function () {
		return this.canvas;
	}

	getContext = function () {
		return this.gl;
	}

	drawRect = function (texture: Texture, pos_x: number, pos_y: number, width: number, height: number) {
		const model = glMatrix.mat4.create();
		glMatrix.mat4.translate(model, model, [pos_x, pos_y, 0.0]);
		glMatrix.mat4.scale(model, model, [width, height, 1.0]);

		this.shader.bind(this.gl);
		this.shader.setP(this.gl, this.projection);
		this.shader.setM(this.gl, model)

		this.gl.bindVertexArray(this.buffer_quad.vao);
		this.gl.bindTexture(this.gl.TEXTURE_2D, texture);
		this.gl.drawArrays(this.gl.TRIANGLES, 0, 6);
	}

	loadTextureImage = function (url: string, callback: (texture: Texture) => void) {
		let gl = this.gl;

		const image = new Image();

		image.onload = function () {
			const tex = gl.createTexture();
			gl.bindTexture(gl.TEXTURE_2D, tex);
			gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);

			let tex_obj = new Texture();
			tex_obj.texture = tex;
			tex_obj.width = image.width;
			tex_obj.height = image.height;

			console.log("Texture " + url + " loaded");
			callback(tex_obj);
		};

		image.src = url;
	}
}

