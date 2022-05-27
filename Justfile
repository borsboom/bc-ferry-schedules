set dotenv-load
set positional-arguments

export AWS_PAGER := ""

schedules_key := "data/schedules.json"
local_schedules_file := "frontend/local/" + schedules_key
upload_data_args := '--output-s3-bucket "$S3_BUCKET" --output-s3-key ' + quote(schedules_key) + ' --invalidate-cloudfront-distribution-id "$CLOUDFRONT_DISTRIBUTION_ID"'

default:
    @{{ just_executable() }} --list

check:
    cargo test
    cargo clippy -- -D warnings
    cargo fmt -- --check
    fix="$(git grep -I -i '@[@]@\|%[%]%\|F[I]XME')"; test "$fix" = "" || (echo "\nFIX COMMENTS:\n$fix\n" >&2; false)
    dirty="$(git status . --short)"; test "$dirty" = "" || (echo "\nDIRTY FILES:\n$dirty\n" >&2; false)
    @echo "\nChecks passed."

local-frontend:
    @if ! test -f {{ quote(local_schedules_file) }}; then echo "\nLocal data not found; run 'just local-data' to scrape it.\n"; false; fi
    cd frontend && trunk serve

local-data *args:
    rm -f {{ quote(local_schedules_file) }}
    mkdir -p {{ quote(parent_directory(local_schedules_file)) }}
    cargo run --bin ferrysched_scraper -- \
        --output-file {{ quote(local_schedules_file) }} \
        "$@"

upload-frontend:
    mkdir -p {{ quote(parent_directory(local_schedules_file)) }}
    cd frontend && trunk build --release --dist dist-release
    @# Work around for the fact that CloudFront does not support auto-compressing wasm files
    wasm="$(ls frontend/dist-release/*.wasm)"; gzip "$wasm" && mv "$wasm.gz" "$wasm"
    aws s3 sync frontend/dist-release/ "s3://$S3_BUCKET/" --acl public-read --delete --exclude "data/*" --exclude "*.wasm" --exclude "*.html" --cache-control max-age=7776000,public
    aws s3 sync frontend/dist-release/ "s3://$S3_BUCKET/" --acl public-read --delete --exclude "*" --include "*.wasm" --cache-control max-age=7776000,public --content-encoding gzip --content-type application/wasm
    aws s3 sync frontend/dist-release/ "s3://$S3_BUCKET/" --acl public-read --delete --exclude "*" --include "*.html" --cache-control max-age=43200,public
    aws cloudfront create-invalidation --distribution-id "$CLOUDFRONT_DISTRIBUTION_ID" --paths "/*"

upload-data *args:
    cargo run --bin ferrysched_scraper -- {{ upload_data_args }} "$@"

upload-data-with-bin bin *args:
    shift; {{ quote(bin) }} {{ upload_data_args }} "$@"

clean:
    rm -rf Cargo.lock frontend/dist/ frontend/dist-release/ frontend/local/ target/
