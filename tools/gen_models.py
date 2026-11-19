"""
Genera los modelos .obj/.mtl del proyecto (nave, moto, porteria, chocolate, arbol).

ESTO ES SOLO UN PUNTO DE PARTIDA: abranlos en Blender y modifiquenlos (cambien
proporciones, agreguen detalles, ajusten colores) para que sean realmente suyos.
Uso:  python tools/gen_models.py     (escribe en assets/models/)
Convenciones: el frente de los modelos mira hacia +Z, arriba es +Y.
"""
import math
import os

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "models")


# ---------------------------------------------------------------- vectores
def sub(a, b): return (a[0]-b[0], a[1]-b[1], a[2]-b[2])
def add(a, b): return (a[0]+b[0], a[1]+b[1], a[2]+b[2])
def mul(a, s): return (a[0]*s, a[1]*s, a[2]*s)
def dot(a, b): return a[0]*b[0] + a[1]*b[1] + a[2]*b[2]
def cross(a, b): return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])
def norm(a):
    l = math.sqrt(dot(a, a)) or 1.0
    return (a[0]/l, a[1]/l, a[2]/l)
def centroid(vs): return mul(tuple(map(sum, zip(*vs))), 1.0/len(vs))


class Model:
    def __init__(self, name, materials):
        self.name, self.materials = name, materials   # materials: {nombre: (kd, ke, ks)}
        self.v, self.f = [], []

    def part(self, verts, faces, mat, ref=None):
        """Agrega una pieza. Voltea los triangulos que queden mirando hacia adentro
        (la pieza debe ser convexa, o dar `ref(centro_de_cara)` con un punto interior)."""
        c = centroid(verts)
        base = len(self.v)
        self.v += verts
        for (i, j, k) in faces:
            a, b, d = verts[i], verts[j], verts[k]
            n = cross(sub(b, a), sub(d, a))
            fc = centroid([a, b, d])
            inner = ref(fc) if ref else c
            if dot(n, sub(fc, inner)) < 0:
                j, k = k, j
            self.f.append((base+i+1, base+j+1, base+k+1, mat))

    def write(self):
        os.makedirs(OUT, exist_ok=True)
        with open(os.path.join(OUT, self.name + ".mtl"), "w") as m:
            for n, (kd, ke, ks) in self.materials.items():
                m.write(f"newmtl {n}\nKd {kd[0]:.3f} {kd[1]:.3f} {kd[2]:.3f}\n"
                        f"Ke {ke[0]:.3f} {ke[1]:.3f} {ke[2]:.3f}\nKs {ks:.3f} {ks:.3f} {ks:.3f}\n\n")
        with open(os.path.join(OUT, self.name + ".obj"), "w") as o:
            o.write(f"# {self.name} (generado por tools/gen_models.py)\nmtllib {self.name}.mtl\no {self.name}\n")
            for v in self.v:
                o.write(f"v {v[0]:.4f} {v[1]:.4f} {v[2]:.4f}\n")
            for mat in self.materials:
                faces = [f for f in self.f if f[3] == mat]
                if faces:
                    o.write(f"usemtl {mat}\n")
                    for a, b, c, _ in faces:
                        o.write(f"f {a} {b} {c}\n")
        print(f"  {self.name}: {len(self.v)} vertices, {len(self.f)} triangulos")


# ------------------------------------------------------------- primitivas
def box(c, s):
    hx, hy, hz = s[0]/2, s[1]/2, s[2]/2
    vs = [(c[0]+x*hx, c[1]+y*hy, c[2]+z*hz) for x in (-1, 1) for y in (-1, 1) for z in (-1, 1)]
    quads = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]
    return vs, [t for q in quads for t in ((q[0], q[1], q[2]), (q[0], q[2], q[3]))]


def frustum(p0, p1, r0, r1, seg=10):
    ax = norm(sub(p1, p0))
    h = (1, 0, 0) if abs(ax[0]) < 0.9 else (0, 1, 0)
    u = norm(cross(ax, h)); w = cross(ax, u)
    vs, fs = [], []
    for p, r in ((p0, r0), (p1, r1)):
        for i in range(seg):
            a = 2*math.pi*i/seg
            vs.append(add(p, add(mul(u, r*math.cos(a)), mul(w, r*math.sin(a)))))
    vs += [p0, p1]
    for i in range(seg):
        j = (i+1) % seg
        fs += [(i, j, seg+j), (i, seg+j, seg+i), (2*seg, j, i), (2*seg+1, seg+i, seg+j)]
    return vs, fs


def ellipsoid(c, r, nu=12, nv=8):
    vs = [(c[0], c[1]+r[1], c[2])]
    for j in range(1, nv):
        t = math.pi*j/nv
        for i in range(nu):
            a = 2*math.pi*i/nu
            vs.append((c[0]+r[0]*math.sin(t)*math.cos(a), c[1]+r[1]*math.cos(t), c[2]+r[2]*math.sin(t)*math.sin(a)))
    vs.append((c[0], c[1]-r[1], c[2]))
    fs = []
    for i in range(nu):
        j = (i+1) % nu
        fs.append((0, 1+i, 1+j))
        fs.append((len(vs)-1, 1+(nv-2)*nu+j, 1+(nv-2)*nu+i))
    for k in range(nv-2):
        for i in range(nu):
            j = (i+1) % nu
            a, b = 1+k*nu+i, 1+k*nu+j
            fs += [(a, a+nu, b+nu), (a, b+nu, b)]
    return vs, fs


def torus_x(c, R, r, nu=16, nv=8):
    """Llanta: anillo alrededor del eje X (para ruedas)."""
    vs, fs = [], []
    for i in range(nu):
        a = 2*math.pi*i/nu
        for j in range(nv):
            b = 2*math.pi*j/nv
            rad = R + r*math.cos(b)
            vs.append((c[0]+r*math.sin(b), c[1]+rad*math.cos(a), c[2]+rad*math.sin(a)))
    for i in range(nu):
        for j in range(nv):
            i2, j2 = (i+1) % nu, (j+1) % nv
            a, b, d, e = i*nv+j, i2*nv+j, i2*nv+j2, i*nv+j2
            fs += [(a, b, d), (a, d, e)]
    def ref(fc):
        y, z = fc[1]-c[1], fc[2]-c[2]
        l = math.hypot(y, z) or 1
        return (c[0], c[1]+R*y/l, c[2]+R*z/l)
    return vs, fs, ref


def lathe(profile, seg=12):
    """Revolucion alrededor del eje Y. profile = [(radio, y), ...] de abajo hacia arriba."""
    vs, fs = [], []
    for (r, y) in profile:
        for i in range(seg):
            a = 2*math.pi*i/seg
            vs.append((r*math.cos(a), y, r*math.sin(a)))
    n = len(profile)
    vs += [(0, profile[0][1], 0), (0, profile[-1][1], 0)]
    for k in range(n-1):
        for i in range(seg):
            j = (i+1) % seg
            a, b = k*seg+i, k*seg+j
            fs += [(a, a+seg, b+seg), (a, b+seg, b)]
    for i in range(seg):
        j = (i+1) % seg
        fs.append((n*seg, j, i))
        fs.append((n*seg+1, (n-1)*seg+i, (n-1)*seg+j))
    return vs, fs


def prism(poly, h0, h1, mapper):
    """Extruye un poligono CONVEXO (a,b) entre h0 y h1. mapper(a,b,h)->(x,y,z)."""
    n = len(poly)
    vs = [mapper(a, b, h0) for a, b in poly] + [mapper(a, b, h1) for a, b in poly]
    fs = []
    for i in range(1, n-1):
        fs += [(0, i, i+1), (n, n+i, n+i+1)]
    for i in range(n):
        j = (i+1) % n
        fs += [(i, j, n+j), (i, n+j, n+i)]
    return vs, fs


# ------------------------------------------------------------------ modelos
def ship():
    m = Model("ship", {
        "hull": ((0.12, 0.33, 0.85), (0, 0, 0), 0.5),
        "accent": ((0.95, 0.10, 0.10), (0, 0, 0), 0.3),
        "dark": ((0.16, 0.17, 0.20), (0, 0, 0), 0.2),
        "glass": ((0.25, 0.65, 0.95), (0.05, 0.15, 0.25), 0.9),
        "glow": ((1.0, 0.45, 0.05), (1.0, 0.45, 0.05), 0.0),
    })
    # Fuselaje: cuerpo trasero + nariz larga y afilada
    m.part(*frustum((0, 0, -1.3), (0, 0, 0.4), 0.32, 0.26, 10), "hull")
    m.part(*frustum((0, 0, 0.4), (0, 0, 1.8), 0.26, 0.03, 10), "hull")
    # Franja de color en la nariz
    m.part(*frustum((0, 0, 1.0), (0, 0, 1.2), 0.17, 0.14, 10), "accent")
    # Cabina de vidrio mas larga
    m.part(*ellipsoid((0, 0.2, 0.25), (0.2, 0.17, 0.6), 12, 8), "glass")
    # Espina dorsal oscura
    m.part(*box((0, 0.32, -0.6), (0.12, 0.10, 1.0)), "dark")

    for s in (-1, 1):
        # Alas en flecha (barridas hacia atras)
        wing = [(0.2*s, -0.3), (1.7*s, -1.25), (1.7*s, -0.9), (0.2*s, 0.5)]
        wing = wing if s > 0 else wing[::-1]
        m.part(*prism(wing, -0.05, 0.04, lambda a, b, h: (a, h, b)), "hull")
        # Franja roja sobre el borde trasero del ala
        stripe = [(1.0*s, -0.807), (1.7*s, -1.25), (1.7*s, -1.0), (1.0*s, -0.55)]
        stripe = stripe if s > 0 else stripe[::-1]
        m.part(*prism(stripe, 0.04, 0.065, lambda a, b, h: (a, h, b)), "accent")
        # Aleta hacia abajo en la punta del ala
        winglet = [(-0.9, 0.0), (-1.25, 0.0), (-1.3, -0.4), (-1.12, -0.4)]
        m.part(*prism(winglet, -0.03, 0.03, lambda a, b, h, s=s: (1.7*s + h, b, a)), "accent")
        # Canards pequenos en la proa
        canard = [(0.12*s, 0.9), (0.65*s, 0.75), (0.65*s, 0.62), (0.12*s, 0.68)]
        canard = canard if s > 0 else canard[::-1]
        m.part(*prism(canard, -0.02, 0.02, lambda a, b, h: (a, h, b)), "dark")
        # Motores laterales
        m.part(*frustum((0.55*s, 0, -0.2), (0.55*s, 0, -1.5), 0.16, 0.18, 10), "dark")
        m.part(*frustum((0.55*s, 0, -1.5), (0.55*s, 0, -1.62), 0.13, 0.09, 10), "glow")

    # Motor central grande
    m.part(*frustum((0, 0, -1.3), (0, 0, -1.65), 0.30, 0.28, 12), "dark")
    m.part(*frustum((0, 0, -1.65), (0, 0, -1.78), 0.22, 0.16, 12), "glow")
    # Aleta dorsal mas alta
    fin = [(-1.3, 0.2), (-0.5, 0.2), (-0.75, 1.0), (-1.2, 1.0)]
    m.part(*prism(fin, -0.03, 0.03, lambda a, b, h: (h, b, a)), "accent")
    m.write()


def moto():
    m = Model("moto", {
        "tire": ((0.06, 0.06, 0.07), (0, 0, 0), 0.1),
        "metal": ((0.70, 0.72, 0.78), (0, 0, 0), 0.9),
        "paint": ((1.00, 0.45, 0.05), (0, 0, 0), 0.6),
        "dark": ((0.10, 0.10, 0.12), (0, 0, 0), 0.2),
        "light": ((1.0, 0.95, 0.7), (1.0, 0.95, 0.7), 0.0),
    })
    for z in (-0.75, 0.75):
        vs, fs, ref = torus_x((0, 0.42, z), 0.30, 0.12, 14, 6)
        m.part(vs, fs, "tire", ref)
        m.part(*frustum((-0.05, 0.42, z), (0.05, 0.42, z), 0.16, 0.16, 8), "metal")
    m.part(*box((0, 0.62, -0.1), (0.16, 0.14, 1.1)), "dark")          # chasis
    m.part(*box((0, 0.50, 0.0), (0.26, 0.26, 0.38)), "metal")          # motor
    m.part(*ellipsoid((0, 0.82, 0.18), (0.16, 0.13, 0.30), 10, 6), "paint")   # tanque
    m.part(*box((0, 0.76, -0.38), (0.17, 0.08, 0.5)), "dark")          # asiento
    m.part(*frustum((0, 0.40, 0.75), (0, 0.96, 0.52), 0.035, 0.035, 6), "metal")  # horquilla
    m.part(*frustum((-0.32, 0.97, 0.52), (0.32, 0.97, 0.52), 0.03, 0.03, 6), "metal")  # manubrio
    m.part(*ellipsoid((0, 0.88, 0.62), (0.08, 0.08, 0.06), 8, 6), "light")  # faro
    m.part(*frustum((0.17, 0.30, -0.1), (0.17, 0.34, -0.95), 0.05, 0.06, 8), "metal")  # escape
    m.write()


def goal():
    m = Model("goal", {
        "post": ((0.95, 0.95, 0.95), (0, 0, 0), 0.6),
        "net": ((0.75, 0.80, 0.85), (0, 0, 0), 0.1),
    })
    for x in (-1.0, 1.0):
        m.part(*frustum((x, 0, 0), (x, 1.0, 0), 0.05, 0.05, 8), "post")
        m.part(*frustum((x, 1.0, 0), (x, 0.9, -0.7), 0.04, 0.04, 8), "post")
        m.part(*frustum((x, 0, 0), (x, 0.0, -0.7), 0.035, 0.035, 8), "post")
    m.part(*frustum((-1.0, 1.0, 0), (1.0, 1.0, 0), 0.05, 0.05, 8), "post")
    m.part(*frustum((-1.0, 0.9, -0.7), (1.0, 0.9, -0.7), 0.04, 0.04, 8), "post")
    # red: solo unas varillas delgadas (marco), para que la porteria se vea abierta
    for x in (-0.5, 0.0, 0.5):
        m.part(*frustum((x, 0.97, -0.05), (x, 0.9, -0.68), 0.012, 0.012, 5), "net")
    m.part(*frustum((-1.0, 0.45, -0.7), (1.0, 0.45, -0.7), 0.012, 0.012, 5), "net")
    m.write()


def kiss():
    m = Model("kiss", {
        "choco": ((0.28, 0.14, 0.08), (0, 0, 0), 0.7),
        "flag": ((0.95, 0.95, 0.92), (0, 0, 0), 0.1),
    })
    prof = [(0.55, 0.0), (0.6, 0.12), (0.55, 0.35), (0.42, 0.6), (0.28, 0.85), (0.15, 1.05), (0.05, 1.18)]
    m.part(*lathe(prof, 12), "choco")
    m.part(*box((0, 1.28, 0), (0.28, 0.1, 0.02)), "flag")
    m.write()


def tree():
    m = Model("tree", {
        "bark": ((0.35, 0.22, 0.12), (0, 0, 0), 0.1),
        "leaf": ((0.12, 0.45, 0.16), (0, 0, 0), 0.2),
        "leaf2": ((0.18, 0.55, 0.20), (0, 0, 0), 0.2),
    })
    m.part(*frustum((0, 0, 0), (0, 0.5, 0), 0.12, 0.09, 7), "bark")
    m.part(*frustum((0, 0.35, 0), (0, 1.0, 0), 0.55, 0.05, 8), "leaf")
    m.part(*frustum((0, 0.75, 0), (0, 1.4, 0), 0.42, 0.04, 8), "leaf2")
    m.part(*frustum((0, 1.15, 0), (0, 1.8, 0), 0.30, 0.02, 8), "leaf")
    m.write()


if __name__ == "__main__":
    print("Generando modelos en assets/models/ ...")
    ship(); moto(); goal(); kiss(); tree()