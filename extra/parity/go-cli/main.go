package main

import (
	"fmt"
	"io/ioutil"
	"os"
	"unicode/utf8"

	"github.com/tidwall/gjson"
)

func main() {
	if len(os.Args) < 3 {
		panic("invalid number of arguments")
	}
	file := os.Args[1]
	path := os.Args[2]
	json, err := ioutil.ReadFile(file)
	if err != nil {
		panic(err)
	}
	raw := gjson.GetBytes(json, path).Raw
	if raw != "" && !gjson.Valid(raw) {
		panic("invalid json response")
	}
	if !utf8.ValidString(raw) {
		panic("invalid utf8 response")
	}
	fmt.Printf("%s\n", raw)
}
