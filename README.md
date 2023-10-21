# article-compiler
article hosting for nerds

## using

### writing articles

edit files in articles/

if a directory looks like this:

```
articles/
  category-0/
    README.md
    some_file.md
  category-1/
    some_other_file.md
```

`{url}/category-0` will display the contents of the README.md file, and `{url}/category-1` will display a directory listing.

subcategories are supported, i.e. 

```
articles/
  category-0/
    category-1/
        category-2/
          some_file.md
```


### customizing

edit files in templates/ to customize

### run

`cargo run --release`

your files should now be in a newly-created `build` folder, if following previous example it should look like such:

```
build/
  category-0/
    index.html
    some_file.html
  category-1/
    some_other_file.html
```
## demo

[Live demo on camper0008-article-compiler.netlify.app](https://camper0008-article-compiler.netlify.app/)

[![Netlify Status](https://api.netlify.com/api/v1/badges/b9b03665-e4f8-4d35-8f85-0053c3a20ff3/deploy-status)](https://app.netlify.com/sites/camper0008-article-compiler/deploys)
