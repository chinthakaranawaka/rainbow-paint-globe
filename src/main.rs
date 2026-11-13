use glfw::{Action, Context, Key};
use std::{ffi::CString, ptr};

const VERTEX_SHADER: &str = r#"#version 330 core
out vec2 uv;
void main() {
    vec2 p = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
    // The oversized triangle spans UV 0..1 across the visible viewport.
    uv = p;
    // Oversize the triangle so it covers every pixel of the viewport.
    gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"#version 330 core
in vec2 uv;
out vec4 color;
uniform float u_time;
uniform vec2 u_resolution;

float hash(vec3 p) {
    p = fract(p * 0.1031);
    p += dot(p, p.yzx + 33.33);
    return fract((p.x + p.y) * p.z);
}

float noise(vec3 p) {
    vec3 i = floor(p);
    vec3 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    float a = mix(hash(i), hash(i + vec3(1.0, 0.0, 0.0)), f.x);
    float b = mix(hash(i + vec3(0.0, 1.0, 0.0)), hash(i + vec3(1.0, 1.0, 0.0)), f.x);
    float c = mix(hash(i + vec3(0.0, 0.0, 1.0)), hash(i + vec3(1.0, 0.0, 1.0)), f.x);
    float d = mix(hash(i + vec3(0.0, 1.0, 1.0)), hash(i + vec3(1.0, 1.0, 1.0)), f.x);
    return mix(mix(a, b, f.y), mix(c, d, f.y), f.z);
}

float fbm(vec3 p) {
    float value = 0.0;
    float amplitude = 0.5;
    for (int i = 0; i < 4; i++) {
        value += amplitude * noise(p);
        p = p * 2.03 + vec3(13.7, 9.2, 5.4);
        amplitude *= 0.5;
    }
    return value;
}

vec3 turn(vec3 p, float angle) {
    float c = cos(angle);
    float s = sin(angle);
    return vec3(c * p.x - s * p.z, p.y, s * p.x + c * p.z);
}

float surface(vec3 p, float t) {
    vec3 flow = turn(p, t * 0.22);
    float swell = fbm(flow * 3.1 + vec3(0.0, t * 0.15, 0.0));
    float detail = noise(flow * 8.0 - vec3(t * 0.2));
    float radius = 1.0 + (swell - 0.5) * 0.20 + (detail - 0.5) * 0.035;
    return length(p) - radius;
}

vec3 surface_normal(vec3 p, float t) {
    float e = 0.002;
    return normalize(vec3(
        surface(p + vec3(e, 0.0, 0.0), t) - surface(p - vec3(e, 0.0, 0.0), t),
        surface(p + vec3(0.0, e, 0.0), t) - surface(p - vec3(0.0, e, 0.0), t),
        surface(p + vec3(0.0, 0.0, e), t) - surface(p - vec3(0.0, 0.0, e), t)
    ));
}

vec3 hsv_to_rgb(vec3 hsv) {
    vec3 rgb = clamp(abs(mod(hsv.x * 6.0 + vec3(0.0, 4.0, 2.0), 6.0) - 3.0) - 1.0, 0.0, 1.0);
    return hsv.z * mix(vec3(1.0), rgb, hsv.y);
}

vec3 paint_color(vec3 p, float t) {
    vec3 q = turn(p, t * 0.22);
    float flow = fbm(q * 3.0 + vec3(0.0, t * 0.18, 0.0));
    float marbling = fbm(q * 7.0 + vec3(0.0, -t * 0.32, 0.0));
    float hue = fract(q.x * 0.78 + q.y * 0.52 + q.z * 0.44
                      + (flow - 0.5) * 1.25 + (marbling - 0.5) * 0.48 + t * 0.055);
    // Keep the pigment fully saturated so the RGB primaries and their blends stay vivid.
    vec3 base = hsv_to_rgb(vec3(hue, 0.96, 0.92));
    float brightVein = smoothstep(0.80, 0.93, marbling);
    base = mix(base, vec3(1.0, 0.88, 0.68), brightVein * 0.09);
    return base;
}

void main() {
    float t = u_time;
    vec2 screen = (uv * 2.0 - 1.0) * vec2(u_resolution.x / u_resolution.y, 1.0);
    vec3 rayOrigin = vec3(0.0, 0.0, 3.2);
    vec3 rayDirection = normalize(vec3(screen * 1.18, -2.25));

    float distanceAlongRay = 0.0;
    float hit = 0.0;
    vec3 point = vec3(0.0);
    for (int i = 0; i < 64; i++) {
        point = rayOrigin + rayDirection * distanceAlongRay;
        float distanceToSurface = surface(point, t);
        if (distanceToSurface < 0.002) {
            hit = 1.0;
            break;
        }
        distanceAlongRay += max(distanceToSurface * 0.72, 0.006);
        if (distanceAlongRay > 6.0) break;
    }

    vec3 background = vec3(0.018, 0.024, 0.05);
    if (hit > 0.5) {
        vec3 normal = surface_normal(point, t);
        vec3 lightDirection = normalize(vec3(-0.45, 0.75, 1.0));
        vec3 viewDirection = normalize(rayOrigin - point);
        vec3 halfDirection = normalize(lightDirection + viewDirection);
        float diffuse = max(dot(normal, lightDirection), 0.0);
        float specular = pow(max(dot(normal, halfDirection), 0.0), 48.0);
        float rim = pow(1.0 - max(dot(normal, viewDirection), 0.0), 3.0);

        vec3 paint = paint_color(point, t);
        vec3 shaded = paint * (0.35 + 0.78 * diffuse);
        shaded += vec3(1.0, 0.9, 0.78) * specular * 0.34;
        shaded += vec3(0.18, 0.38, 0.95) * rim * 0.30;
        float fog = exp(-distanceAlongRay * 0.035);
        color = vec4(mix(background, shaded, fog), 1.0);
    } else {
        color = vec4(background, 1.0);
    }
}
"#;

fn compile_shader(kind: u32, source: &str) -> Result<u32, String> {
    let shader;
    unsafe {
        shader = gl::CreateShader(kind);
        let source = CString::new(source).expect("Shader source contains no null bytes");
        let source_ptr = source.as_ptr();
        gl::ShaderSource(shader, 1, &source_ptr, ptr::null());
        gl::CompileShader(shader);

        let mut success = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        if success == 0 {
            let mut length = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut length);
            let mut log = vec![0_u8; length.max(1) as usize];
            gl::GetShaderInfoLog(shader, length, ptr::null_mut(), log.as_mut_ptr().cast());
            gl::DeleteShader(shader);
            return Err(String::from_utf8_lossy(&log).trim_end_matches('\0').to_owned());
        }
    }
    Ok(shader)
}

fn create_program() -> Result<u32, String> {
    let vertex = compile_shader(gl::VERTEX_SHADER, VERTEX_SHADER)?;
    let fragment = match compile_shader(gl::FRAGMENT_SHADER, FRAGMENT_SHADER) {
        Ok(shader) => shader,
        Err(error) => {
            unsafe { gl::DeleteShader(vertex) };
            return Err(error);
        }
    };

    let program;
    unsafe {
        program = gl::CreateProgram();
        gl::AttachShader(program, vertex);
        gl::AttachShader(program, fragment);
        gl::LinkProgram(program);
        gl::DeleteShader(vertex);
        gl::DeleteShader(fragment);

        let mut success = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
        if success == 0 {
            let mut length = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut length);
            let mut log = vec![0_u8; length.max(1) as usize];
            gl::GetProgramInfoLog(program, length, ptr::null_mut(), log.as_mut_ptr().cast());
            gl::DeleteProgram(program);
            return Err(String::from_utf8_lossy(&log).trim_end_matches('\0').to_owned());
        }
    }
    Ok(program)
}

fn main() {
    let mut glfw = glfw::init(glfw::fail_on_errors).expect("Failed to initialize GLFW");
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));

    let (mut window, events) = glfw
        .create_window(900, 650, "Rainbow Paint Mixing - Rust OpenGL", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window");

    window.make_current();
    window.set_key_polling(true);
    glfw.set_swap_interval(glfw::SwapInterval::Sync(1));

    gl::load_with(|symbol| {
        window
            .get_proc_address(symbol)
            .map_or(ptr::null(), |function| function as *const _)
    });

    let program = create_program().expect("Failed to create OpenGL shader program");
    let time_name = CString::new("u_time").unwrap();
    let resolution_name = CString::new("u_resolution").unwrap();
    let time_location;
    let resolution_location;
    let mut vao = 0;
    unsafe {
        time_location = gl::GetUniformLocation(program, time_name.as_ptr());
        resolution_location = gl::GetUniformLocation(program, resolution_name.as_ptr());
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);
        gl::UseProgram(program);
    }

    while !window.should_close() {
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            if let glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) = event {
                window.set_should_close(true);
            }
        }

        let (width, height) = window.get_framebuffer_size();
        unsafe {
            gl::Viewport(0, 0, width, height);
            gl::UseProgram(program);
            gl::Uniform1f(time_location, glfw.get_time() as f32);
            gl::Uniform2f(resolution_location, width as f32, height as f32);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
        }
        window.swap_buffers();
    }

    unsafe {
        gl::DeleteVertexArrays(1, &vao);
        gl::DeleteProgram(program);
    }
}

