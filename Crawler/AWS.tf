# AWS.tf

variable "region" {
  type    = string
  default = "ap-northeast-2"
}

variable "bucket_name" {
  type    = string
  default = "uring-announcements"
}

variable "s3_prefix" {
  type    = string
  default = "v1"
}

variable "lambda_zip_path" {
  type    = string
  default = "../target/lambda/lambda/bootstrap.zip"
}

variable "schedule_expression" {
  type    = string
  default = "rate(10 minutes)"
}

# Set AWS Provider
provider "aws" {
  region = var.region
}

# S3 Bucket
resource "aws_s3_bucket" "data_bucket" {
  bucket = var.bucket_name
}

# IAM Role
resource "aws_iam_role" "lambda_exec" {
  name = "crawler_lambda_role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = { Service = "lambda.amazonaws.com" }
    }]
  })
}

# IAM Policy for S3 Access
resource "aws_iam_role_policy" "lambda_s3_access" {
  role = aws_iam_role.lambda_exec.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Action = [
        "s3:PutObject",
        "s3:GetObject",
        "s3:DeleteObject"
      ]
      Resource = "${aws_s3_bucket.data_bucket.arn}/*"
    }, {
      Effect   = "Allow"
      Action   = ["s3:ListBucket"]
      Resource = aws_s3_bucket.data_bucket.arn
    }]
  })
}

# Lambda Basic Execution Role (CloudWatch Logs)
resource "aws_iam_role_policy_attachment" "lambda_basic_execution" {
  role       = aws_iam_role.lambda_exec.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

# Lambda Function
resource "aws_lambda_function" "crawler_func" {
  function_name = "yonsei-crawler-lambda"
  role          = aws_iam_role.lambda_exec.arn
  handler       = "bootstrap"         # Rust Lambda always uses bootstrap
  runtime       = "provided.al2023"   # Amazon Linux 2023 (Custom Runtime)
  architectures = ["arm64"]           # Graviton Processor (cost saving)
  timeout       = 120                 # Adjust based on crawling time

  # Built Zip file path (moved to this path by GitHub Actions)
  filename         = var.lambda_zip_path
  source_code_hash = filebase64sha256(var.lambda_zip_path)
  
  environment {
    variables = {
      RUST_LOG       = "info"
      S3_BUCKET      = aws_s3_bucket.data_bucket.id
      S3_PREFIX      = var.s3_prefix
    }
  }
}

# EventBridge schedule (10-minute interval)
resource "aws_cloudwatch_event_rule" "crawler_schedule" {
  name                = "uring-crawler-schedule"
  schedule_expression = var.schedule_expression
}

resource "aws_cloudwatch_event_target" "crawler_target" {
  rule      = aws_cloudwatch_event_rule.crawler_schedule.name
  target_id = "crawler-lambda"
  arn       = aws_lambda_function.crawler_func.arn
}

resource "aws_lambda_permission" "allow_eventbridge" {
  statement_id  = "AllowExecutionFromEventBridge"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.crawler_func.function_name
  principal     = "events.amazonaws.com"
  source_arn    = aws_cloudwatch_event_rule.crawler_schedule.arn
}
