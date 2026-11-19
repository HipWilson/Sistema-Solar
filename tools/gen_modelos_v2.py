"""
Version 2 de los modelos de superficie: arbol, moto, porteria y kiss.
Reusa las piezas de tools/gen_models.py. Mantiene tamano, origen en la base y orientacion
(frente +Z, arriba +Y) para que encajen en el juego sin tocar el codigo Rust.

Uso (desde la raiz del repo):
    python tools/gen_modelos_v2.py
Escribe assets/models/{tree,moto,goal,kiss}.obj/.mtl

ESTE SCRIPT SOLO GENERA UN PUNTO DE PARTIDA (hecho con ayuda de IA): abre los .obj en Blender,
modificalos y guarda tus .blend en assets/blend/.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_models import Model, box, frustum, ellipsoid, torus_x, lathe, prism  # noqa: E402


def tree():
    """Arbol frondoso con tres masas de hojas, raices y dos manzanas."""
    m = Model("tree", {
        "bark": ((0.35, 0.22, 0.12), (0, 0, 0), 0.1),
        "leaf": ((0.12, 0.45, 0.16), (0, 0, 0), 0.2),
        "leaf2": ((0.20, 0.58, 0.22), (0, 0, 0), 0.2),
        "fruit": ((0.90, 0.10, 0.10), (0, 0, 0), 0.5),
    })
    m.part(*frustum((0, 0, 0), (0, 0.85, 0), 0.15, 0.09, 7), "bark")           # tronco
    m.part(*frustum((0, 0, 0), (0.18, 0.12, 0.05), 0.07, 0.03, 5), "bark")     # raiz
    m.part(*frustum((0, 0, 0), (-0.15, 0.10, -0.08), 0.07, 0.03, 5), "bark")   # raiz
    m.part(*ellipsoid((0, 1.15, 0), (0.58, 0.48, 0.58), 10, 5), "leaf")        # copa principal
    m.part(*ellipsoid((0.28, 1.45, 0.08), (0.36, 0.32, 0.36), 10, 5), "leaf2")  # copa alta
    m.part(*ellipsoid((-0.28, 1.38, -0.10), (0.32, 0.30, 0.32), 10, 5), "leaf")
    m.part(*ellipsoid((0.40, 1.00, 0.38), (0.07, 0.07, 0.07), 6, 4), "fruit")  # manzanas
    m.part(*ellipsoid((-0.42, 1.12, 0.20), (0.07, 0.07, 0.07), 6, 4), "fruit")
    m.write()


def moto():
    """Moto con piloto, casco, guardabarros y luz trasera."""
    m = Model("moto", {
        "tire": ((0.06, 0.06, 0.07), (0, 0, 0), 0.1),
        "metal": ((0.70, 0.72, 0.78), (0, 0, 0), 0.9),
        "paint": ((1.00, 0.45, 0.05), (0, 0, 0), 0.6),
        "dark": ((0.10, 0.10, 0.12), (0, 0, 0), 0.2),
        "light": ((1.0, 0.95, 0.7), (1.0, 0.95, 0.7), 0.0),
        "suit": ((0.10, 0.25, 0.80), (0, 0, 0), 0.3),
        "helmet": ((0.90, 0.90, 0.95), (0, 0, 0), 0.8),
        "tail": ((1.0, 0.10, 0.05), (0.9, 0.08, 0.04), 0.0),
    })
    for z in (-0.75, 0.75):
        vs, fs, ref = torus_x((0, 0.42, z), 0.30, 0.12, 14, 6)
        m.part(vs, fs, "tire", ref)
        m.part(*frustum((-0.05, 0.42, z), (0.05, 0.42, z), 0.16, 0.16, 8), "metal")
    m.part(*box((0, 0.62, -0.1), (0.16, 0.14, 1.1)), "dark")                       # chasis
    m.part(*box((0, 0.50, 0.0), (0.26, 0.26, 0.38)), "metal")                       # motor
    m.part(*ellipsoid((0, 0.82, 0.18), (0.16, 0.13, 0.30), 10, 6), "paint")         # tanque
    m.part(*box((0, 0.76, -0.38), (0.17, 0.08, 0.5)), "dark")                       # asiento
    m.part(*frustum((0, 0.40, 0.75), (0, 0.96, 0.52), 0.035, 0.035, 6), "metal")    # horquilla
    m.part(*frustum((-0.32, 0.97, 0.52), (0.32, 0.97, 0.52), 0.03, 0.03, 6), "metal")  # manubrio
    m.part(*ellipsoid((0, 0.88, 0.62), (0.08, 0.08, 0.06), 8, 6), "light")          # faro
    m.part(*frustum((0.17, 0.30, -0.1), (0.17, 0.34, -0.95), 0.05, 0.06, 8), "metal")  # escape
    # --- nuevo: piloto
    m.part(*box((0, 1.08, -0.30), (0.30, 0.46, 0.22)), "suit")                      # torso
    m.part(*ellipsoid((0, 1.42, -0.18), (0.16, 0.17, 0.19), 10, 6), "helmet")       # casco
    m.part(*ellipsoid((0, 1.42, -0.04), (0.10, 0.07, 0.04), 8, 4), "dark")          # visor
    for s in (-1, 1):
        m.part(*frustum((0.16*s, 1.18, -0.28), (0.30*s, 0.97, 0.50), 0.045, 0.04, 6), "suit")   # brazos
        m.part(*frustum((0.10*s, 0.90, -0.34), (0.14*s, 0.62, 0.05), 0.07, 0.05, 6), "suit")    # piernas
    # --- nuevo: guardabarros y luz trasera
    m.part(*box((0, 0.72, -0.98), (0.14, 0.04, 0.40)), "paint")
    m.part(*box((0, 0.70, -1.17), (0.10, 0.05, 0.03)), "tail")
    m.write()


def goal():
    """Porteria con banderines en las esquinas y travesano rojo."""
    m = Model("goal", {
        "post": ((0.95, 0.95, 0.95), (0, 0, 0), 0.6),
        "net": ((0.75, 0.80, 0.85), (0, 0, 0), 0.1),
        "flag": ((0.95, 0.15, 0.10), (0, 0, 0), 0.2),
    })
    for x in (-1.0, 1.0):
        m.part(*frustum((x, 0, 0), (x, 1.0, 0), 0.05, 0.05, 8), "post")
        m.part(*frustum((x, 1.0, 0), (x, 0.9, -0.7), 0.04, 0.04, 8), "post")
        m.part(*frustum((x, 0, 0), (x, 0.0, -0.7), 0.035, 0.035, 8), "post")
    m.part(*frustum((-1.0, 1.0, 0), (1.0, 1.0, 0), 0.05, 0.05, 8), "post")
    m.part(*frustum((-1.0, 0.9, -0.7), (1.0, 0.9, -0.7), 0.04, 0.04, 8), "post")
    for x in (-0.5, 0.0, 0.5):
        m.part(*frustum((x, 0.97, -0.05), (x, 0.9, -0.68), 0.012, 0.012, 5), "net")
    m.part(*frustum((-1.0, 0.45, -0.7), (1.0, 0.45, -0.7), 0.012, 0.012, 5), "net")
    # --- nuevo: banderines de esquina y red lateral
    for s in (-1, 1):
        m.part(*frustum((1.15*s, 0, 0.15), (1.15*s, 0.75, 0.15), 0.02, 0.015, 6), "post")
        tri = [(0.0, 0.0), (0.0, 0.28), (0.22, 0.14)]
        tri = tri if s > 0 else tri[::-1]
        m.part(*prism(tri, -0.01, 0.01, lambda a, b, h, s=s: (1.15*s + h, 0.45 + b, 0.15 + a*s)), "flag")
        m.part(*frustum((1.0*s, 0.5, -0.02), (1.0*s, 0.5, -0.68), 0.012, 0.012, 5), "net")
    m.write()


def kiss():
    """Chocolate kiss con envoltorio plateado en la base y bandera con franja."""
    m = Model("kiss", {
        "choco": ((0.28, 0.14, 0.08), (0, 0, 0), 0.7),
        "flag": ((0.95, 0.95, 0.92), (0, 0, 0), 0.1),
        "foil": ((0.75, 0.78, 0.85), (0, 0, 0), 0.9),
        "stripe": ((0.85, 0.12, 0.12), (0, 0, 0), 0.1),
    })
    prof = [(0.55, 0.0), (0.6, 0.12), (0.55, 0.35), (0.42, 0.6), (0.28, 0.85), (0.15, 1.05), (0.05, 1.18)]
    m.part(*lathe(prof, 12), "choco")
    m.part(*frustum((0, 0.0, 0), (0, 0.08, 0), 0.64, 0.64, 12), "foil")             # base plateada
    m.part(*frustum((0, 1.14, 0), (0, 1.40, 0), 0.015, 0.015, 5), "foil")            # mastil
    m.part(*box((0.14, 1.30, 0), (0.28, 0.12, 0.02)), "flag")                       # bandera
    m.part(*box((0.14, 1.30, 0), (0.28, 0.04, 0.024)), "stripe")                    # franja
    m.write()


if __name__ == "__main__":
    print("Generando modelos v2 en assets/models/ ...")
    tree(); moto(); goal(); kiss()