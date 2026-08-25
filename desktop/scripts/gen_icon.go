//go:build ignore

// gen_icon generates a 1024x1024 source icon for the Quintodrome app.
// Run: go run desktop/scripts/gen_icon.go
package main

import (
	"image"
	"image/color"
	"image/png"
	"os"
)

const size = 1024
const ss = 4 // supersampling factor per axis

func lerp(a, b, t float64) float64 { return a + (b-a)*t }

type rgb struct{ r, g, b float64 }

func mix(a, b rgb, t float64) rgb {
	return rgb{lerp(a.r, b.r, t), lerp(a.g, b.g, t), lerp(a.b, b.b, t)}
}

var (
	bgTop        = rgb{58, 42, 94}
	bgBottom     = rgb{23, 21, 28}
	circleTop    = rgb{168, 85, 247}
	circleBottom = rgb{124, 58, 237}
	white        = rgb{255, 255, 255}
)

func inTriangle(px, py, ax, ay, bx, by, cx, cy float64) bool {
	sign := func(x1, y1, x2, y2, x3, y3 float64) float64 {
		return (x1-x3)*(y2-y3) - (x2-x3)*(y1-y3)
	}
	d1 := sign(px, py, ax, ay, bx, by)
	d2 := sign(px, py, bx, by, cx, cy)
	d3 := sign(px, py, cx, cy, ax, ay)
	neg := d1 < 0 || d2 < 0 || d3 < 0
	pos := d1 > 0 || d2 > 0 || d3 > 0
	return !(neg && pos)
}

func sample(x, y float64) rgb {
	// background vertical gradient
	c := mix(bgTop, bgBottom, y/size)

	// centered circle
	cx, cy, cr := float64(size)/2, float64(size)/2, float64(size)*0.31
	dx, dy := x-cx, y-cy
	if d := dx*dx + dy*dy; d <= cr*cr {
		t := (y - (cy - cr)) / (2 * cr)
		circle := mix(circleTop, circleBottom, t)
		c = circle
	}

	// play triangle
	tx := float64(size) * 0.512
	ax, ay := tx-float64(size)*0.145, float64(size)*0.342
	bx, by := tx-float64(size)*0.145, float64(size)*0.658
	cxv, cyv := tx+float64(size)*0.165, float64(size)*0.5
	if inTriangle(x, y, ax, ay, bx, by, cxv, cyv) {
		c = white
	}

	return c
}

func main() {
	img := image.NewRGBA(image.Rect(0, 0, size, size))
	for y := 0; y < size; y++ {
		for x := 0; x < size; x++ {
			var acc rgb
			for sy := 0; sy < ss; sy++ {
				for sx := 0; sx < ss; sx++ {
					fx := float64(x) + (float64(sx)+0.5)/ss
					fy := float64(y) + (float64(sy)+0.5)/ss
					s := sample(fx, fy)
					acc.r += s.r
					acc.g += s.g
					acc.b += s.b
				}
			}
			n := float64(ss * ss)
			img.SetRGBA(x, y, color.RGBA{
				R: uint8(acc.r / n),
				G: uint8(acc.g / n),
				B: uint8(acc.b / n),
				A: 255,
			})
		}
	}

	f, err := os.Create(os.Args[len(os.Args)-1])
	if err != nil {
		panic(err)
	}
	defer f.Close()
	if err := png.Encode(f, img); err != nil {
		panic(err)
	}
}
